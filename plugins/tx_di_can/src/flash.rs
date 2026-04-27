//! UDS 刷写引擎
//!
//! ## 标准 UDS 刷写流程
//! ```text
//! ┌─ 1. 安全访问 (0x27) ────────────────────────┐
//! │   requestSeed → keyFn(seed) → verifyKey    │
//! └────────────────────────────────────────────┘
//! ┌─ 2. 进入编程会话 (0x10 03) ─────────────────┐
//! └────────────────────────────────────────────┘
//! ┌─ 3. 请求下载 (0x34) ────────────────────────┐
//! │   告知 ECU: 目标地址 + 文件大小              │
//! │   ECU 返回 maxBlockSize                    │
//! └────────────────────────────────────────────┘
//! ┌─ 4. 分块传输 (0x36) ───────────────────────┐
//! │   每块: blockSeq 0x01..0xFF wrap            │
//! │   每块大小 ≤ maxBlockSize (含 1 字节 header) │
//! └────────────────────────────────────────────┘
//! ┌─ 5. 退出传输 (0x37) ───────────────────────┐
//! └────────────────────────────────────────────┘
//! ┌─ 6. 例程控制: 检查完整性 (0x31 01 xx) ──────┐
//! └────────────────────────────────────────────┘
//! ┌─ 7. ECU 复位 (0x11 01) ─────────────────────┐
//! └────────────────────────────────────────────┘
//! ```
//!
//! ## 擦除策略
//! - **0xFF 填充区域**：自动跳过，避免额外擦除开销
//! - **预擦除**：可通过 `erase_before_download=true` 配置显式全擦

use crate::event::{emit_event, CanEvent};
use crate::uds::{SessionType, UdsClient};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 刷写配置
#[derive(Debug, Clone)]
pub struct FlashConfig {
    /// ECU 物理寻址 ID（发送 CAN ID）
    pub target_id: u32,
    /// 安全访问等级（奇数，请求 seed）
    pub security_level: u8,
    /// 编程会话类型（默认 0x02 编程会话；部分 ECU 用 0x03 扩展+特殊）
    pub session_type: SessionType,
    /// 每块数据最大字节数（ECU 返回值；若为 0 则使用此值）
    pub default_block_size: usize,
    /// 是否在下载前擦除（传 0xFF 区域的块是否跳过）
    pub erase_before_download: bool,
    /// 校验例程 ID（0x31 01 xx 中的 xx）
    pub verify_routine_id: u8,
    /// 目标内存起始地址（下载到哪里）
    pub memory_address: u32,
    /// 目标内存格式标识符
    pub memory_size_len: u8,
    /// 例程控制选项数据（hex 原始字节）
    pub routine_option: Vec<u8>,
}

impl Default for FlashConfig {
    fn default() -> Self {
        FlashConfig {
            target_id: 0x7E0,
            security_level: 0x01,
            session_type: SessionType::Programming,
            default_block_size: 4096,
            erase_before_download: false,
            verify_routine_id: 0x02,
            memory_address: 0x00000000,
            memory_size_len: 4,
            routine_option: vec![],
        }
    }
}

/// 刷写进度
#[derive(Debug, Clone)]
pub struct FlashProgress {
    pub block_seq: u32,
    pub total_blocks: u32,
    pub bytes_sent: usize,
    pub total_bytes: usize,
    pub elapsed_ms: u64,
}

/// 刷写结果
#[derive(Debug)]
pub struct FlashResult {
    pub total_bytes: usize,
    pub elapsed_ms: u64,
}

/// 刷写引擎
pub struct FlashEngine {
    uds: Arc<UdsClient>,
    config: FlashConfig,
    /// 当前块序号（0x36 的 blockSeqCounter，1-based）
    block_seq: AtomicU32,
    bytes_sent: AtomicUsize,
    start: std::sync::Mutex<Option<Instant>>,
}

impl FlashEngine {
    pub fn new(uds: Arc<UdsClient>, config: FlashConfig) -> Self {
        FlashEngine {
            uds,
            config,
            block_seq: AtomicU32::new(1),
            bytes_sent: AtomicUsize::new(0),
            start: std::sync::Mutex::new(None),
        }
    }

    /// 执行完整刷写流程
    ///
    /// `key_fn`：ECU 安全算法回调（seed → key）
    pub async fn flash<F>(
        &self,
        firmware: impl AsRef<Path>,
        key_fn: F,
    ) -> Result<FlashResult>
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync,
    {
        let data = std::fs::read(firmware.as_ref())
            .map_err(|e| anyhow!("读取固件文件失败: {}", e))?;

        self.flash_data(data, key_fn).await
    }

    /// 直接刷写内存中的固件数据
    pub async fn flash_data<F>(&self, data: Vec<u8>, key_fn: F) -> Result<FlashResult>
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync,
    {
        let total_bytes = data.len();
        let total_blocks = ((total_bytes as f64)
            / (self.config.default_block_size as f64 - 1.0))
            .ceil() as u32;

        {
            let mut guard = self.start.lock().unwrap();
            *guard = Some(Instant::now());
        }

        tracing::info!(
            "[flash] 刷写开始: {} bytes, 预计 {} 块, target=0x{:X}",
            total_bytes,
            total_blocks,
            self.config.target_id
        );

        // ── 1. 安全访问 ──────────────────────────────
        tracing::info!("[flash] 1/7 安全访问 (level={})", self.config.security_level);
        self.uds
            .security_access(self.config.security_level, &key_fn)
            .await
            .map_err(|e| anyhow!("安全访问失败: {e}"))?;

        // ── 2. 进入编程会话 ──────────────────────────
        tracing::info!(
            "[flash] 2/7 进入编程会话 ({:?})",
            self.config.session_type
        );
        self.uds
            .session_control(self.config.session_type)
            .await
            .map_err(|e| anyhow!("进入编程会话失败: {e}"))?;

        // ── 3. 请求下载 0x34 ─────────────────────────
        tracing::info!(
            "[flash] 3/7 请求下载 (addr=0x{:08X}, len={})",
            self.config.memory_address,
            total_bytes
        );
        let max_block_size = self
            .uds
            .request_download(
                self.config.memory_address,
                total_bytes as u32,
                0x00, // 不压缩
                0x00, // 不加密
                self.config.memory_size_len,
                self.config.memory_size_len,
            )
            .await
            .map_err(|e| anyhow!("请求下载失败: {e}"))?;

        let block_size = if max_block_size > 0 {
            max_block_size
        } else {
            self.config.default_block_size
        };
        tracing::info!(
            "[flash] ECU maxBlockSize={} bytes, 使用块大小={}",
            max_block_size,
            block_size
        );

        // ── 4. 分块传输 0x36 ─────────────────────────
        tracing::info!("[flash] 4/7 分块传输 (块大小={})", block_size);
        self.transfer_blocks(&data, block_size).await?;

        // ── 5. 退出传输 0x37 ─────────────────────────
        tracing::info!("[flash] 5/7 退出传输");
        self.uds
            .request_transfer_exit()
            .await
            .map_err(|e| anyhow!("退出传输失败: {e}"))?;

        // ── 6. 例程控制：校验完整性 ──────────────────
        tracing::info!(
            "[flash] 6/7 完整性校验 (routine=0x{:02X})",
            self.config.verify_routine_id
        );
        let _ = self
            .uds
            .routine_control(
                0x01, // startRoutine
                0xE0 << 8 | self.config.verify_routine_id as u16,
                &self.config.routine_option,
            )
            .await;

        // ── 7. ECU 复位 ──────────────────────────────
        tracing::info!("[flash] 7/7 ECU 复位 (0x11 01)");
        self.uds
            .ecu_reset(0x01) // hardReset
            .await
            .map_err(|e| anyhow!("ECU 复位失败: {e}"))?;

        let elapsed_ms = {
            let guard = self.start.lock().unwrap();
            guard.map(|i| i.elapsed().as_millis() as u64).unwrap_or(0)
        };

        tracing::info!(
            "[flash] ✅ 刷写完成: {} bytes / {} ms",
            total_bytes,
            elapsed_ms
        );
        emit_event(CanEvent::FlashComplete {
            total_bytes,
            elapsed_ms,
        })
        .await;

        Ok(FlashResult {
            total_bytes,
            elapsed_ms,
        })
    }

    /// 分块传输
    async fn transfer_blocks(&self, data: &[u8], block_size: usize) -> Result<()> {
        let total_bytes = data.len();
        let total_blocks = ((total_bytes as f64) / (block_size as f64)).ceil() as u32;

        let mut offset = 0usize;
        // blockSeq 从 1 开始
        let mut seq: u8 = 1;

        while offset < total_bytes {
            // 跳过全 0xFF 块（已擦除区域无需重写）
            let chunk_end = (offset + block_size).min(total_bytes);
            let chunk = &data[offset..chunk_end];

            if self.config.erase_before_download || !is_all_ff(chunk) {
                let resp = self
                    .uds
                    .transfer_data(seq, chunk)
                    .await
                    .map_err(|e| anyhow!("TransferData block {} 失败: {e}", seq))?;

                // 某些 ECU 会回传额外数据（如校验结果），可忽略
                if !resp.is_empty() {
                    tracing::debug!(
                        "[flash] block {} resp: {:?}",
                        seq,
                        resp.iter().take(8).collect::<Vec<_>>()
                    );
                }
            }

            self.block_seq.store(seq as u32, Ordering::Relaxed);
            let sent = self.bytes_sent.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
            let _elapsed_ms = {
                let guard = self.start.lock().unwrap();
                guard.map(|i| i.elapsed().as_millis() as u64).unwrap_or(0)
            };

            // 发进度事件
            emit_event(CanEvent::FlashProgress {
                block_seq: seq as u32,
                total_blocks,
                bytes_sent: sent,
                total_bytes,
            })
            .await;

            tracing::debug!(
                "[flash] [{}/{}] block {} sent {} bytes",
                seq,
                total_blocks,
                seq,
                chunk.len()
            );

            offset = chunk_end;
            seq = seq.wrapping_add(1);
        }
        Ok(())
    }
}

/// 判断数据块是否全为 0xFF（已擦除 flash）
pub(crate) fn is_all_ff(data: &[u8]) -> bool {
    data.iter().all(|&b| b == 0xFF)
}
