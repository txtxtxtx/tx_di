//! ECU 仿真节点（通用可配置）
//!
//! 当使用 SimBus（或显式开启 `sim_ecu`）时，在插件层启动本任务：
//! 订阅适配器接收通道 → ISO-TP 重组得到 UDS 请求 → 按描述库生成响应 →
//! 经适配器回发，使 SimBus 从"回环"变为"ECU 应答"。对真实适配器零侵入。
//!
//! 仿真节点作为订阅者挂接在 `CanPlugin` 的 `start_rx_loop` 之后，复用了
//! `isotp.rs` 与 `uds.rs` 的编解码逻辑，不重复实现传输层。

pub mod seedkey;
pub mod state;

use crate::adapter::CanAdapter;
use crate::db::DescDb;
use crate::isotp::{IsoTpChannel, IsoTpConfig};
use state::SimEcuState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// ECU 仿真配置
#[derive(Debug, Clone)]
pub struct SimEcuConfig {
    /// 诊断请求 CAN ID（tester→ECU，通常 0x7E0）
    pub req_id: u32,
    /// 诊断响应 CAN ID（ECU→tester，通常 0x7E8）
    pub resp_id: u32,
    /// 是否使用 CAN-FD 传输
    pub is_fd: bool,
    /// ECU 是否要求先解锁安全访问才能下载（bootloader 校验）
    pub require_security_for_flash: bool,
    /// ECU 是否要求编程会话才能下载
    pub require_programming_session: bool,
}

impl SimEcuConfig {
    /// 由 `CanConfig` 推导默认仿真配置
    pub fn from_can_config(adapter: &crate::config::CanConfig) -> Self {
        SimEcuConfig {
            req_id: adapter.isotp_tx_id,
            resp_id: adapter.isotp_rx_id,
            is_fd: adapter.enable_fd,
            require_security_for_flash: true,
            require_programming_session: true,
        }
    }
}

/// 启动 ECU 仿真后台任务
///
/// - `adapter`：当前总线适配器（SimBus 等）
/// - `cfg`：仿真节点配置
/// - `db`：描述库（应答 DID/DTC 内容来源）
/// - `running`：运行标志，置 false 时优雅退出
///
/// 返回 `JoinHandle` 供调用方跟踪；任务随 `running=false` 自动结束。
pub fn spawn_sim_ecu(
    adapter: Arc<dyn CanAdapter>,
    cfg: SimEcuConfig,
    db: Arc<DescDb>,
    running: Arc<AtomicBool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let isotp_cfg = IsoTpConfig {
            tx_id: cfg.resp_id, // ECU 发送用响应 ID
            rx_id: cfg.req_id,  // ECU 接收用请求 ID
            is_fd: cfg.is_fd,
            ..Default::default()
        };
        let channel = IsoTpChannel::new(adapter, isotp_cfg);
        let mut state = SimEcuState::new(&db);
        state.require_security_for_flash = cfg.require_security_for_flash;
        state.require_programming_session = cfg.require_programming_session;

        tracing::info!(
            "[sim_ecu] 仿真节点已启动 (req=0x{:X}, resp=0x{:X}, fd={})",
            cfg.req_id,
            cfg.resp_id,
            cfg.is_fd
        );

        loop {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            // 1s 超时以便检查运行标志
            match channel.recv(1000).await {
                Ok(req) => {
                    if req.is_empty() {
                        continue;
                    }
                    let resp = state.handle(&req);
                    if let Err(e) = channel.send(&resp).await {
                        tracing::warn!("[sim_ecu] 响应发送失败: {e}");
                    }
                }
                Err(_) => {
                    // 超时：继续循环以检查 running 标志
                    continue;
                }
            }
        }
        tracing::info!("[sim_ecu] 仿真节点已退出");
    })
}

#[cfg(test)]
mod tests;
