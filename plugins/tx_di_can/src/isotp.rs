//! ISO-TP (ISO 15765-2) 传输层实现
//!
//! ## 帧类型
//! - **SF** (0x0_): 单帧
//!   - Classical CAN：最大 7 字节
//!   - CAN-FD：最大 62 字节；超过 62 走 escape 序列（最长 4095 字节）
//! - **FF** (0x1_): 首帧，开始多帧传输
//!   - Classical CAN：12-bit 长度，首帧携带 6 字节
//!   - CAN-FD：12-bit 长度（首帧携带 62 字节）；超过 4095 走 32-bit escape
//! - **CF** (0x2_): 连续帧
//!   - Classical CAN：每帧 7 字节
//!   - CAN-FD：每帧 63 字节
//! - **FC** (0x3_): 流控帧，ECU 控制发送速率
//!
//! ## 流控状态
//! - CTS (0x00)：继续发送
//! - WAIT (0x01)：等待
//! - OVFLW (0x02)：溢出，中止
//!
//! ## FD escape 序列（ISO 15765-2:2016）
//! - 单帧 escape：PCI=0x00，随后 0x00 + 2 字节（大端）长度，数据从偏移 4 起。
//! - 首帧 escape：PCI=0x10 0x00，随后 4 字节（大端）32-bit 长度，数据从偏移 6 起。

use crate::adapter::CanAdapter;
use crate::frame::{CanFdFrame, CanFrame, FrameId, FrameKind};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

/// CAN-FD 单帧（非 escape）最大数据长度
const FD_SF_MAX: usize = 62;
/// CAN-FD 首帧（非 escape）首段携带数据长度（64 - 2 PCI 字节）
const FD_FF_DATA: usize = 62;
/// CAN-FD 首帧 escape 首段携带数据长度（64 - 6 PCI 字节）
const FD_FF_ESC_DATA: usize = 58;
/// CAN-FD 连续帧每帧携带数据长度（64 - 1 PCI 字节）
const FD_CF_DATA: usize = 63;

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
    /// 是否使用 CAN-FD 传输（FD 帧 + 64 字节 + escape 序列）
    pub is_fd: bool,
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
            is_fd: false,
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

    /// 发送任意长度数据（自动分帧；FD 模式走 FD 帧与 64 字节路径）
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        if self.config.is_fd {
            self.send_fd(data).await
        } else {
            self.send_classical(data).await
        }
    }

    /// 接收完整的 ISO-TP 消息（阻塞到 p2_timeout_ms）
    pub async fn recv(&self, timeout_ms: u64) -> Result<Vec<u8>> {
        if self.config.is_fd {
            self.recv_fd(timeout_ms).await
        } else {
            self.recv_classical(timeout_ms).await
        }
    }

    // ─────────────────────────────────────────────
    // Classical CAN 发送
    // ─────────────────────────────────────────────

    async fn send_classical(&self, data: &[u8]) -> Result<()> {
        let len = data.len();
        if len <= 7 {
            self.send_classical_single(data).await
        } else {
            self.send_classical_multi(data).await
        }
    }

    /// 发送单帧 SF（Classical）
    async fn send_classical_single(&self, data: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; if self.config.enable_padding { 8 } else { data.len() + 1 }];
        buf[0] = data.len() as u8; // PCI: SF, DL
        buf[1..=data.len()].copy_from_slice(data);
        if self.config.enable_padding {
            for b in &mut buf[data.len() + 1..] {
                *b = self.config.padding_byte;
            }
        }
        let frame = self.make_classical_frame(buf);
        self.adapter.send(&frame).await
    }

    /// 多帧发送（FF + CF + FC 流控，Classical）
    async fn send_classical_multi(&self, data: &[u8]) -> Result<()> {
        let total = data.len();
        let mut rx = self.adapter.subscribe();

        // 首帧 FF
        let ff_dl_hi = ((total >> 8) & 0x0F) as u8;
        let ff_dl_lo = (total & 0xFF) as u8;
        let mut ff = vec![0u8; 8];
        ff[0] = 0x10 | ff_dl_hi;
        ff[1] = ff_dl_lo;
        ff[2..8].copy_from_slice(&data[..6]);
        self.adapter.send(&self.make_classical_frame(ff)).await?;

        let mut offset = 6usize;
        let mut sn: u8 = 1;

        loop {
            // 等待 FC 帧
            let fc = self.recv_classical_fc(&mut rx).await?;
            let fs = fc[0] & 0x0F;
            let bs = fc[1];
            let st_ms = fc[2];
            let st_delay = if st_ms > 0 { st_ms as u64 } else { self.config.st_min_ms as u64 };

            match fs {
                0x00 => { /* CTS：继续 */ }
                0x01 => {
                    // WAIT：稍后重试
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
                self.adapter.send(&self.make_classical_frame(cf)).await?;

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
    // Classical CAN 接收
    // ─────────────────────────────────────────────

    async fn recv_classical(&self, timeout_ms: u64) -> Result<Vec<u8>> {
        let mut rx = self.adapter.subscribe();
        timeout(
            Duration::from_millis(timeout_ms),
            self.recv_classical_inner(&mut rx),
        )
        .await
        .map_err(|_| anyhow!("ISO-TP: 接收超时 ({}ms)", timeout_ms))?
    }

    async fn recv_classical_inner(
        &self,
        rx: &mut broadcast::Receiver<CanFrame>,
    ) -> Result<Vec<u8>> {
        let rx_id = self.config.rx_id;
        loop {
            let frame = self.next_classical_frame(rx, rx_id).await?;
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
                    self.send_classical_fc(0x00, self.config.block_size, self.config.st_min_ms)
                        .await?;

                    let mut expected_sn: u8 = 1;
                    while buf.len() < total {
                        let cf = self.next_classical_frame(rx, rx_id).await?;
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
    // CAN-FD 发送
    // ─────────────────────────────────────────────

    async fn send_fd(&self, data: &[u8]) -> Result<()> {
        let len = data.len();
        if len == 0 {
            return Err(anyhow!("ISO-TP FD: 空数据无法发送"));
        }
        if len <= FD_SF_MAX {
            self.send_fd_single(data).await
        } else {
            self.send_fd_multi(data).await
        }
    }

    /// 发送 FD 单帧（含 escape 序列处理）
    ///
    /// FD 单帧长度编码（ISO 15765-2:2016）。CAN-FD 帧最大 64 字节，故单帧最多携带 62 字节：
    /// - 1..=15 字节：PCI = 0x0 | DL（DL 占低 4 位）
    /// - 16..=62 字节：escape 1 字节，0x00 <DL>
    /// > 62 字节由调用方（`send_fd`）路由到多帧路径，不会进入本函数。
    async fn send_fd_single(&self, data: &[u8]) -> Result<()> {
        let len = data.len();
        if len <= 15 {
            let mut buf = vec![0u8; len + 1];
            buf[0] = len as u8;
            buf[1..].copy_from_slice(data);
            self.adapter.send_fd(&self.make_fd_frame(buf)).await
        } else if len <= FD_SF_MAX {
            let mut buf = vec![0u8; len + 2];
            buf[0] = 0x00;
            buf[1] = len as u8;
            buf[2..].copy_from_slice(data);
            self.adapter.send_fd(&self.make_fd_frame(buf)).await
        } else {
            Err(anyhow!(
                "ISO-TP FD: 单帧数据过长 ({}B)，应走多帧路径",
                len
            ))
        }
    }

    /// FD 多帧发送（FF + CF + FC 流控）
    async fn send_fd_multi(&self, data: &[u8]) -> Result<()> {
        let total = data.len();
        let mut rx = self.adapter.subscribe_fd();

        // 首帧 FF（normal 或 escape）
        let (data_start, first) = if total <= 0x0FFF {
            let mut ff = vec![0u8; 2];
            ff[0] = 0x10 | ((total >> 8) & 0x0F) as u8;
            ff[1] = (total & 0xFF) as u8;
            (2usize, FD_FF_DATA.min(total))
        } else {
            let mut ff = vec![0u8; 6];
            ff[0] = 0x10;
            ff[1] = 0x00;
            ff[2] = ((total >> 24) & 0xFF) as u8;
            ff[3] = ((total >> 16) & 0xFF) as u8;
            ff[4] = ((total >> 8) & 0xFF) as u8;
            ff[5] = (total & 0xFF) as u8;
            (6usize, FD_FF_ESC_DATA.min(total))
        };
        let mut ff = Vec::with_capacity(data_start + first);
        if data_start == 2 {
            ff.push(0x10 | ((total >> 8) & 0x0F) as u8);
            ff.push((total & 0xFF) as u8);
        } else {
            ff.push(0x10);
            ff.push(0x00);
            ff.push(((total >> 24) & 0xFF) as u8);
            ff.push(((total >> 16) & 0xFF) as u8);
            ff.push(((total >> 8) & 0xFF) as u8);
            ff.push((total & 0xFF) as u8);
        }
        ff.extend_from_slice(&data[..first]);
        self.adapter.send_fd(&self.make_fd_frame(ff)).await?;

        let mut offset = first;
        let mut sn: u8 = 1;

        loop {
            let fc = self.recv_fd_fc(&mut rx).await?;
            let fs = fc[0] & 0x0F;
            let bs = fc[1];
            let st_ms = fc[2];
            let st_delay = if st_ms > 0 { st_ms as u64 } else { self.config.st_min_ms as u64 };

            match fs {
                0x00 => { /* CTS */ }
                0x01 => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    continue;
                }
                0x02 => return Err(anyhow!("ISO-TP FD: ECU 缓冲区溢出 (FC OVFLW)")),
                _ => return Err(anyhow!("ISO-TP FD: 未知 FC FS={:02X}", fs)),
            }

            let mut block_cnt = 0u8;
            while offset < total {
                let chunk_end = (offset + FD_CF_DATA).min(total);
                let chunk = &data[offset..chunk_end];

                let mut cf = vec![0u8; chunk.len() + 1];
                cf[0] = 0x20 | (sn & 0x0F);
                cf[1..].copy_from_slice(chunk);
                self.adapter.send_fd(&self.make_fd_frame(cf)).await?;

                sn = sn.wrapping_add(1);
                offset = chunk_end;
                block_cnt = block_cnt.wrapping_add(1);

                if st_delay > 0 {
                    tokio::time::sleep(Duration::from_millis(st_delay)).await;
                }

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
    // CAN-FD 接收
    // ─────────────────────────────────────────────

    async fn recv_fd(&self, timeout_ms: u64) -> Result<Vec<u8>> {
        let mut rx = self.adapter.subscribe_fd();
        timeout(
            Duration::from_millis(timeout_ms),
            self.recv_fd_inner(&mut rx),
        )
        .await
        .map_err(|_| anyhow!("ISO-TP FD: 接收超时 ({}ms)", timeout_ms))?
    }

    async fn recv_fd_inner(
        &self,
        rx: &mut broadcast::Receiver<CanFdFrame>,
    ) -> Result<Vec<u8>> {
        let rx_id = self.config.rx_id;
        loop {
            let frame = self.next_fd_frame(rx, rx_id).await?;
            let data = &frame.data;
            if data.is_empty() {
                continue;
            }
            let pci_type = (data[0] & 0xF0) >> 4;

            match pci_type {
                // 单帧（含 escape）
                0x0 => {
                    let dl = (data[0] & 0x0F) as usize;
                    let payload = if dl == 0 {
                        // escape 序列
                        if data.len() < 2 {
                            return Err(anyhow!("ISO-TP FD: SF escape 截断"));
                        }
                        if data[1] == 0x00 {
                            if data.len() < 4 {
                                return Err(anyhow!("ISO-TP FD: SF escape 长度截断"));
                            }
                            let len =
                                ((data[2] as usize) << 8) | (data[3] as usize);
                            let start = 4;
                            if data.len() < start + len {
                                return Err(anyhow!("ISO-TP FD: SF escape 数据不足"));
                            }
                            data[start..start + len].to_vec()
                        } else {
                            let len = data[1] as usize;
                            if data.len() < 2 + len {
                                return Err(anyhow!("ISO-TP FD: SF escape 数据不足"));
                            }
                            data[2..2 + len].to_vec()
                        }
                    } else {
                        // 普通 FD 单帧：DL 占低 4 位（1..=15）
                        if data.len() < 1 + dl {
                            return Err(anyhow!("ISO-TP FD: SF 数据不足"));
                        }
                        data[1..1 + dl].to_vec()
                    };
                    return Ok(payload);
                }
                // 首帧（含 escape）
                // FD FF escape 标识：data[0]==0x10 且 data[1]==0x00（共 4 字节大端长度）。
                // 注意：普通 FF 当总长 ≤ 255 时 data[0] 也等于 0x10（高 4 位为 0），
                // 因此不能仅以 data[0]&0x0F==0 判断 escape，必须以 data[1]==0x00 区分。
                0x1 => {
                    let (total, data_start) = if data[0] == 0x10 && data[1] == 0x00 {
                        if data.len() < 6 {
                            return Err(anyhow!("ISO-TP FD: FF escape 截断"));
                        }
                        let total = u32::from_be_bytes([data[2], data[3], data[4], data[5]])
                            as usize;
                        (total, 6usize)
                    } else {
                        let total =
                            (((data[0] & 0x0F) as usize) << 8) | (data[1] as usize);
                        if total < 1 {
                            return Err(anyhow!("ISO-TP FD: FF_DL 过小 ({})", total));
                        }
                        (total, 2usize)
                    };

                    let mut buf = Vec::with_capacity(total);
                    let first = data.len().saturating_sub(data_start).min(total);
                    buf.extend_from_slice(&data[data_start..data_start + first]);

                    // 发送 FC CTS
                    self.send_fd_fc(0x00, self.config.block_size, self.config.st_min_ms)
                        .await?;

                    let mut expected_sn: u8 = 1;
                    while buf.len() < total {
                        let cf = self.next_fd_frame(rx, rx_id).await?;
                        let cf_data = &cf.data;
                        let cf_type = (cf_data[0] & 0xF0) >> 4;
                        if cf_type != 0x2 {
                            return Err(anyhow!("ISO-TP FD: 期望 CF，收到 {:02X}", cf_data[0]));
                        }
                        let sn = cf_data[0] & 0x0F;
                        if sn != (expected_sn & 0x0F) {
                            return Err(anyhow!(
                                "ISO-TP FD: SN 不连续，期望 {}, 收到 {}",
                                expected_sn & 0x0F,
                                sn
                            ));
                        }
                        let remaining = total - buf.len();
                        let take = (cf_data.len() - 1).min(remaining);
                        buf.extend_from_slice(&cf_data[1..1 + take]);
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
    // 辅助（Classical）
    // ─────────────────────────────────────────────

    fn make_classical_frame(&self, data: Vec<u8>) -> CanFrame {
        CanFrame {
            id: if self.config.extended_id {
                FrameId::Extended(self.config.tx_id)
            } else {
                FrameId::from_raw(self.config.tx_id)
            },
            kind: FrameKind::Data,
            data,
            timestamp_us: 0,
        }
    }

    async fn send_classical_fc(&self, fs: u8, bs: u8, st_min: u8) -> Result<()> {
        let mut buf = vec![self.config.padding_byte; 8];
        buf[0] = 0x30 | (fs & 0x0F);
        buf[1] = bs;
        buf[2] = st_min;
        self.adapter.send(&self.make_classical_frame(buf)).await
    }

    async fn recv_classical_fc(
        &self,
        rx: &mut broadcast::Receiver<CanFrame>,
    ) -> Result<Vec<u8>> {
        let rx_id = self.config.rx_id;
        let frame = timeout(Duration::from_millis(1000), self.next_classical_frame(rx, rx_id))
            .await
            .map_err(|_| anyhow!("ISO-TP: 等待 FC 超时"))??;
        Ok(frame.data)
    }

    async fn next_classical_frame(
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

    // ─────────────────────────────────────────────
    // 辅助（CAN-FD）
    // ─────────────────────────────────────────────

    fn make_fd_frame(&self, data: Vec<u8>) -> CanFdFrame {
        CanFdFrame {
            id: if self.config.extended_id {
                FrameId::Extended(self.config.tx_id)
            } else {
                FrameId::from_raw(self.config.tx_id)
            },
            data,
            brs: false,
            esi: false,
            timestamp_us: 0,
        }
    }

    async fn send_fd_fc(&self, fs: u8, bs: u8, st_min: u8) -> Result<()> {
        let mut buf = vec![0u8; 3];
        buf[0] = 0x30 | (fs & 0x0F);
        buf[1] = bs;
        buf[2] = st_min;
        self.adapter.send_fd(&self.make_fd_frame(buf)).await
    }

    async fn recv_fd_fc(
        &self,
        rx: &mut broadcast::Receiver<CanFdFrame>,
    ) -> Result<Vec<u8>> {
        let rx_id = self.config.rx_id;
        let frame = timeout(Duration::from_millis(1000), self.next_fd_frame(rx, rx_id))
            .await
            .map_err(|_| anyhow!("ISO-TP FD: 等待 FC 超时"))??;
        Ok(frame.data)
    }

    async fn next_fd_frame(
        &self,
        rx: &mut broadcast::Receiver<CanFdFrame>,
        filter_id: u32,
    ) -> Result<CanFdFrame> {
        loop {
            match rx.recv().await {
                Ok(f) => {
                    if f.id.raw() == filter_id { return Ok(f); }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("[isotp] FD 接收滞后丢弃 {} 帧", n);
                    continue;
                }
                Err(e) => return Err(anyhow!("ISO-TP FD: 接收通道关闭: {}", e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::SimBusAdapter;
    use std::time::Duration as StdDuration;
    use tokio::sync::broadcast;

    // 测试辅助：使用调用方预先订阅的接收通道做完整 ISO-TP 重组。
    // 必须在对端发送前完成订阅（broadcast 仅投递订阅之后发出的消息）。
    #[cfg(test)]
    impl IsoTpChannel {
        async fn recv_on_classical(
            &self,
            mut rx: broadcast::Receiver<CanFrame>,
            timeout_ms: u64,
        ) -> Result<Vec<u8>> {
            timeout(Duration::from_millis(timeout_ms), self.recv_classical_inner(&mut rx))
                .await
                .map_err(|_| anyhow!("ISO-TP: 接收超时 ({}ms)", timeout_ms))?
        }

        async fn recv_on_fd(
            &self,
            mut rx: broadcast::Receiver<CanFdFrame>,
            timeout_ms: u64,
        ) -> Result<Vec<u8>> {
            timeout(Duration::from_millis(timeout_ms), self.recv_fd_inner(&mut rx))
                .await
                .map_err(|_| anyhow!("ISO-TP FD: 接收超时 ({}ms)", timeout_ms))?
        }
    }

    // 单帧 round-trip（Classical）
    #[tokio::test]
    async fn classical_single_frame() {
        let adapter = Arc::new(SimBusAdapter::new("t", 128));
        let tester = IsoTpChannel::new(
            adapter.clone(),
            IsoTpConfig { tx_id: 0x7E0, rx_id: 0x7E8, ..Default::default() },
        );
        let ecu = IsoTpChannel::new(
            adapter.clone(),
            IsoTpConfig { tx_id: 0x7E8, rx_id: 0x7E0, ..Default::default() },
        );

        // 先订阅 ECU 接收通道，确保不丢首帧
        let ecu_rx = adapter.subscribe();
        tokio::spawn(async move {
            if let Ok(req) = ecu.recv_on_classical(ecu_rx, 1000).await {
                let resp: Vec<u8> = req.iter().map(|b| b.wrapping_add(1)).collect();
                let _ = ecu.send(&resp).await;
            }
        });

        let payload = vec![0x22, 0xF1, 0x90];
        tester.send(&payload).await.unwrap();
        let got = tester.recv(1000).await.unwrap();
        assert_eq!(got, vec![0x23, 0xF2, 0x91]);
    }

    // 多帧（classical + FD）通用 round-trip 测试
    // ECU 端做完整 ISO-TP 接收（含流控交互）后，将整段请求回显（+1）作为整段响应
    async fn round_trip(is_fd: bool, len: usize) {
        let adapter = Arc::new(SimBusAdapter::new("t", 512));
        let tester = IsoTpChannel::new(
            adapter.clone(),
            IsoTpConfig { tx_id: 0x7E0, rx_id: 0x7E8, is_fd, ..Default::default() },
        );
        let ecu = IsoTpChannel::new(
            adapter.clone(),
            IsoTpConfig { tx_id: 0x7E8, rx_id: 0x7E0, is_fd, ..Default::default() },
        );

        // 先订阅 ECU 接收通道（classical / FD），确保不丢首帧
        if is_fd {
            let ecu_rx = adapter.subscribe_fd();
            tokio::spawn(async move {
                if let Ok(req) = ecu.recv_on_fd(ecu_rx, 3000).await {
                    let resp: Vec<u8> = req.iter().map(|b| b.wrapping_add(1)).collect();
                    let _ = ecu.send(&resp).await;
                }
            });
        } else {
            let ecu_rx = adapter.subscribe();
            tokio::spawn(async move {
                if let Ok(req) = ecu.recv_on_classical(ecu_rx, 3000).await {
                    let resp: Vec<u8> = req.iter().map(|b| b.wrapping_add(1)).collect();
                    let _ = ecu.send(&resp).await;
                }
            });
        }

        let payload: Vec<u8> = (0..len).map(|i| i as u8).collect();
        tester.send(&payload).await.unwrap();
        let got = tester.recv(3000).await.unwrap();
        let expected: Vec<u8> = payload.iter().map(|b| b.wrapping_add(1)).collect();
        assert_eq!(got, expected);
    }

    #[tokio::test]
    async fn classical_multi_frame() {
        // 40 字节 → 多帧（Classical）
        round_trip(false, 40).await;
    }

    #[tokio::test]
    async fn fd_single_frame() {
        // 62 字节 → FD 单帧
        round_trip(true, 62).await;
    }

    #[tokio::test]
    async fn fd_single_frame_escape() {
        // 200 字节 → FD 单帧 escape 序列
        round_trip(true, 200).await;
    }

    #[tokio::test]
    async fn fd_multi_frame() {
        // 500 字节 → FD 多帧（normal FF + CF）
        round_trip(true, 500).await;
    }

    #[tokio::test]
    async fn fd_multi_frame_escape() {
        // 5000 字节 → FD 多帧（escape FF + CF）
        round_trip(true, 5000).await;
    }

    #[tokio::test]
    async fn fd_timing() {
        // 确认 FD 路径不抛错且在合理时间内完成
        let start = std::time::Instant::now();
        round_trip(true, 300).await;
        assert!(start.elapsed() < StdDuration::from_secs(2));
    }
}
