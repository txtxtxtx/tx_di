//! GB28181 事件总线
//!
//! 提供类型安全的事件订阅/发布机制，上层业务通过 `Gb28181Server::on_event()` 订阅。
//! 内部使用 `tokio::sync::broadcast` 实现多消费者扇出。

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, RwLock};

/// GB28181 事件类型
#[derive(Debug, Clone)]
pub enum Gb28181Event {
    /// 设备注册上线
    DeviceRegistered {
        device_id: String,
        contact: String,
        remote_addr: String,
    },

    /// 设备主动注销
    DeviceUnregistered { device_id: String },

    /// 设备心跳超时离线
    DeviceOffline { device_id: String },

    /// 设备重新上线（心跳恢复）
    DeviceOnline { device_id: String },

    /// 收到设备心跳
    Keepalive {
        device_id: String,
        status: String,
    },

    /// 收到目录响应（通道列表）
    CatalogReceived {
        device_id: String,
        channel_count: usize,
    },

    /// 点播会话建立（ACK 后）
    SessionStarted {
        device_id: String,
        channel_id: String,
        call_id: String,
    },

    /// 点播会话结束
    SessionEnded {
        device_id: String,
        channel_id: String,
        call_id: String,
    },

    /// 收到设备信息响应
    DeviceInfoReceived {
        device_id: String,
        manufacturer: String,
        model: String,
        firmware: String,
    },
}

/// 异步事件处理函数类型
pub type Gb28181EventHandler = Arc<
    dyn Fn(Gb28181Event) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>
        + Send
        + Sync,
>;

/// 全局事件监听器列表
static EVENT_LISTENERS: LazyLock<Arc<RwLock<Vec<Gb28181EventHandler>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// 注册事件监听器
pub fn add_event_listener<F, Fut>(handler: F)
where
    F: Fn(Gb28181Event) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let handler_fn: Gb28181EventHandler =
        Arc::new(move |ev| Box::pin(handler(ev)));
    EVENT_LISTENERS
        .write()
        .expect("event listener write lock")
        .push(handler_fn);
}

/// 发布事件（广播给所有监听器）
///
/// 在 `tokio::spawn` 内部调用，保证不阻塞 SIP 处理循环。
pub async fn emit(event: Gb28181Event) {
    let listeners = {
        EVENT_LISTENERS
            .read()
            .expect("event listener read lock")
            .clone()
    };
    for listener in listeners.iter() {
        if let Err(e) = listener(event.clone()).await {
            tracing::warn!(error = %e, "GB28181 事件处理器返回错误");
        }
    }
}

/// 清空所有监听器（测试用）
#[allow(dead_code)]
pub fn clear_listeners() {
    EVENT_LISTENERS
        .write()
        .expect("event listener write lock")
        .clear();
}
