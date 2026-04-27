//! ISO-TP (ISO 15765-2) 传输层实现
//!
//! ## 帧类型
//! - **SF** (0x0_): 单帧，最大 7 字节（Classical CAN）或 62 字节（CANFD escape）
//! - **FF** (0x1_): 首帧，开始多帧传输
//! - **CF** (0x2_): 连续帧，携带后续数据
//! - **FC** (0x3_): 流控帧，ECU 控制发送速率
//!
//! ## 流控状态
//! - CTS (0x00)：继续发送
//! - WAIT (0x01)：等待
//! - OVFLW (0x02)：溢出，中止

use crate::adapter::CanAdapter;
use crate::frame::{CanFrame, FrameId};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

/// ISO-TP 通道配置
#[derive(Debug, Clone)]
pub struct IsoTpConfig {
    /// 发送 CAN ID（上位机→ECU）
    pub tx_id: u32,
    /// 接收 CAN ID（ECU→上位机）
    pub rx_id: u32,
    /// 块大小（BS；0 = 不限制，ECU 决定）
    pub block_size: u8,
    /// 帧间隔 ST_min（ms）
    pub st_min_ms: u8,
    /// 单帧/帧段填充字节（0xCC 或 0x00）
    pub padding_byte: u8,
    /// 是否启用填充（Classical CAN 需要填到 8 字节）
    pub enable_padding: bool,
    /// 是否使用扩展帧 ID
    pub extended_id: bool,
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        IsoTpConfig {
            tx_id: 0x7E0,
            rx_id: 0x7E8,
            block_size: 0,
            st_min_ms: 0,
            padding_byte: 0xCC,
            enable_padding: true,
            extended_id: false,
        }
    }
}

/// ISO-TP 通道（单个 tx/rx ID 对）
pub struct IsoTpChannel {
    config: IsoTpConfig,
    adapter: Arc<dyn CanAdapter>,
}

impl IsoTpChannel {
    pub fn new(adapter: Arc<dyn CanAdapter>, config: IsoTpConfig) -> Self {
        IsoTpChannel { config, adapter }
    }

    // ─────────────────────────────────────────────
    // 发送
    // ─────────────────────────────────────────────

    /// 发送任意长度数据（自动分帧）
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let len = data.len();
        if len <= 7 {
            self.send_single_frame(data).await
        } else {
            self.send_multi_frame(data).await
        }
    }

    /// 发送单帧 SF
    async fn send_single_frame(&self, data: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; if self.config.enable_padding { 8 } else { data.len() + 1 }];
        buf[0] = data.len() as u8; // PCI: SF, DL
        buf[1..=data.len()].copy_from_slice(data);
        if self.config.enable_padding {
            for b in &mut buf[data.len() + 1..] {
                *b = self.config.padding_byte;
            }
        }
        let frame = self.make_frame(buf);
        self.adapter.send(&frame).await
    }

    /// 多帧发送（FF + CF + FC 流控）
    async fn send_multi_frame(&self, data: &[u8]) -> Result<()> {
        let total = data.len();
        let mut rx = self.adapter.subscribe();

        // 首帧 FF
        let ff_dl_hi = ((total >> 8) & 0x0F) as u8;
        let ff_dl_lo = (total & 0xFF) as u8;
        let mut ff = vec![0u8; 8];
        ff[0] = 0x10 | ff_dl_hi;
        ff[1] = ff_dl_lo;
        ff[2..8].copy_from_slice(&data[..6]);
        self.adapter.send(&self.make_frame(ff)).await?;

        let mut offset = 6usize;
        let mut sn: u8 = 1;

        loop {
            // 等待 FC 帧
            let fc = self.recv_fc(&mut rx).await?;
            let fs = fc[0] & 0x0F;
            let bs = fc[1];
            let st_ms = fc[2];
            let st_delay = if st_ms > 0 { st_ms as u64 } else { self.config.st_min_ms as u64 };

            match fs {
                0x00 => { /* CTS：继续 */ }
                0x01 => {
                    // WAIT：稍后重试（最多等 3 次）
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    continue;
                }
                0x02 => return Err(anyhow!("ISO-TP: ECU 缓冲区溢出 (FC OVFLW)")),
                _ => return Err(anyhow!("ISO-TP: 未知 FC FS={:02X}", fs)),
            }

            let mut block_cnt = 0u8;
            while offset < total {
                let chunk_end = (offset + 7).min(total);
                let chunk = &data[offset..chunk_end];

                let mut cf = vec![self.config.padding_byte; 8];
                cf[0] = 0x20 | (sn & 0x0F);
                cf[1..=chunk.len()].copy_from_slice(chunk);
                self.adapter.send(&self.make_frame(cf)).await?;

                sn = sn.wrapping_add(1);
                offset = chunk_end;
                block_cnt = block_cnt.wrapping_add(1);

                if st_delay > 0 {
                    tokio::time::sleep(Duration::from_millis(st_delay)).await;
                }

                // 达到 BS 限制，需要下一个 FC
                if bs > 0 && block_cnt >= bs {
                    break;
                }
            }

            if offset >= total {
                break;
            }
        }
        Ok(())
    }

    // ─────────────────────────────────────────────
    // 接收
    // ─────────────────────────────────────────────

    /// 接收完整的 ISO-TP 消息（阻塞到 p2_timeout_ms）
    pub async fn recv(&self, timeout_ms: u64) -> Result<Vec<u8>> {
        let mut rx = self.adapter.subscribe();
        timeout(
            Duration::from_millis(timeout_ms),
            self.recv_inner(&mut rx),
        )
        .await
        .map_err(|_| anyhow!("ISO-TP: 接收超时 ({}ms)", timeout_ms))?
    }

    async fn recv_inner(&self, rx: &mut broadcast::Receiver<CanFrame>) -> Result<Vec<u8>> {
        let rx_id = self.config.rx_id;
        loop {
            let frame = self.next_frame(rx, rx_id).await?;
            let pci_type = (frame.data[0] & 0xF0) >> 4;

            match pci_type {
                // 单帧
                0x0 => {
                    let dl = (frame.data[0] & 0x0F) as usize;
                    if dl == 0 || dl > 7 {
                        return Err(anyhow!("ISO-TP: SF 无效 DL={}", dl));
                    }
                    return Ok(frame.data[1..=dl].to_vec());
                }
                // 首帧
                0x1 => {
                    let total = (((frame.data[0] & 0x0F) as usize) << 8)
                        | frame.data[1] as usize;
                    let mut buf = Vec::with_capacity(total);
                    buf.extend_from_slice(&frame.data[2..8.min(frame.data.len())]);

                    // 发送 FC CTS
                    self.send_fc(0x00, self.config.block_size, self.config.st_min_ms)
                        .await?;

                    let mut expected_sn: u8 = 1;
                    while buf.len() < total {
                        let cf = self.next_frame(rx, rx_id).await?;
                        let cf_type = (cf.data[0] & 0xF0) >> 4;
                        if cf_type != 0x2 {
                            return Err(anyhow!("ISO-TP: 期望 CF，收到 {:02X}", cf.data[0]));
                        }
                        let sn = cf.data[0] & 0x0F;
                        if sn != (expected_sn & 0x0F) {
                            return Err(anyhow!("ISO-TP: SN 不连续，期望 {}, 收到 {}", expected_sn & 0x0F, sn));
                        }
                        let remaining = total - buf.len();
                        let take = 7usize.min(remaining);
                        buf.extend_from_slice(&cf.data[1..=take]);
                        expected_sn = expected_sn.wrapping_add(1);
                    }
                    return Ok(buf[..total].to_vec());
                }
                _ => {
                    // 忽略 FC 和其他帧（等待 SF/FF）
                    continue;
                }
            }
        }
    }

    // ─────────────────────────────────────────────
    // 辅助
    // ─────────────────────────────────────────────

    fn make_frame(&self, data: Vec<u8>) -> CanFrame {
        CanFrame {
            id: if self.config.extended_id {
                FrameId::Extended(self.config.tx_id)
            } else {
                FrameId::from_raw(self.config.tx_id)
            },
            kind: crate::frame::FrameKind::Data,
            data,
            timestamp_us: 0,
        }
    }

    async fn send_fc(&self, fs: u8, bs: u8, st_min: u8) -> Result<()> {
        let mut buf = vec![self.config.padding_byte; 8];
        buf[0] = 0x30 | (fs & 0x0F);
        buf[1] = bs;
        buf[2] = st_min;
        self.adapter.send(&self.make_frame(buf)).await
    }

    async fn recv_fc(&self, rx: &mut broadcast::Receiver<CanFrame>) -> Result<Vec<u8>> {
        let rx_id = self.config.rx_id;
        let frame = timeout(Duration::from_millis(1000), self.next_frame(rx, rx_id))
            .await
            .map_err(|_| anyhow!("ISO-TP: 等待 FC 超时"))??;
        Ok(frame.data)
    }

    async fn next_frame(
        &self,
        rx: &mut broadcast::Receiver<CanFrame>,
        filter_id: u32,
    ) -> Result<CanFrame> {
        loop {
            match rx.recv().await {
                Ok(f) if f.id.raw() == filter_id => return Ok(f),
                Ok(_) => continue, // 过滤其他 ID
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("[isotp] 接收滞后丢弃 {} 帧", n);
                    continue;
                }
                Err(e) => return Err(anyhow!("ISO-TP: 接收通道关闭: {}", e)),
            }
        }
    }
}
