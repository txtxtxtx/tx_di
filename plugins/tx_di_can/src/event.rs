//! CAN 事件总线
//!
//! 基于 `LazyLock<Vec<Handler>>` 实现多订阅者异步扇出，
//! 与 GB28181 插件相同的模式，保持架构一致性。

use crate::frame::{CanFdFrame, CanFrame};
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};

/// CAN 插件事件
#[derive(Debug, Clone)]
pub enum CanEvent {
    /// 总线初始化完成
    BusReady {
        interface: String,
    },
    /// 总线错误（被动/主动错误、Bus Off 等）
    BusError {
        description: String,
    },
    /// 接收到标准 CAN 帧（经过接收过滤器后推送）
    FrameReceived(CanFrame),
    /// 接收到 CANFD 帧
    FdFrameReceived(CanFdFrame),
    /// 发送帧完成
    FrameSent {
        id: u32,
        len: usize,
    },
    /// ISO-TP 收到完整的多帧消息（已重组）
    IsoTpReceived {
        tx_id: u32,
        rx_id: u32,
        data: Vec<u8>,
    },
    /// UDS 请求已发出
    UdsRequest {
        service: u8,
        payload: Vec<u8>,
    },
    /// UDS 正响应
    UdsResponse {
        service: u8,
        payload: Vec<u8>,
    },
    /// UDS 负响应（NRC）
    UdsNegativeResponse {
        service: u8,
        nrc: u8,
    },
    /// UDS 超时
    UdsTimeout {
        service: u8,
    },
    /// 刷写进度更新
    FlashProgress {
        /// 当前已传输块序号
        block_seq: u32,
        /// 总块数（估算）
        total_blocks: u32,
        /// 已传输字节数
        bytes_sent: usize,
        /// 总字节数
        total_bytes: usize,
    },
    /// 刷写完成
    FlashComplete {
        /// 固件文件大小（字节）
        total_bytes: usize,
        /// 耗时（ms）
        elapsed_ms: u64,
    },
    /// 刷写失败
    FlashError {
        reason: String,
    },
}

pub type AsyncHandler = Box<
    dyn Fn(CanEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>
        + Send
        + Sync,
>;

static HANDLERS: LazyLock<Mutex<Vec<AsyncHandler>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// 注册事件处理器（可在 `BuildContext::build()` 之前调用）
pub fn on_event<F, Fut>(handler: F)
where
    F: Fn(CanEvent) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let boxed: AsyncHandler = Box::new(move |ev| Box::pin(handler(ev)));
    HANDLERS.lock().unwrap().push(boxed);
}

/// 内部：向所有订阅者广播事件
pub async fn emit_event(ev: CanEvent) {
    let handlers: Vec<_> = {
        // 只持有锁期间克隆 handler 列表（函数指针 + Arc，极低开销）
        HANDLERS
            .lock()
            .unwrap()
            .iter()
            .map(|h| {
                // SAFETY: handler 是 Send+Sync，克隆事件并调用
                let ev2 = ev.clone();
                h(ev2)
            })
            .collect()
    };
    for fut in handlers {
        if let Err(e) = fut.await {
            tracing::warn!("[can] event handler error: {e}");
        }
    }
}
