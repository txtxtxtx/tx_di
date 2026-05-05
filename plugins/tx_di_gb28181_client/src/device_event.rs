//! 设备端事件总线

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, RwLock};

/// GB28181 设备端事件
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    /// 注册成功
    Registered { platform_uri: String },

    /// 注册失败（将重试）
    RegisterFailed { reason: String, retry_in_secs: u64 },

    /// 注销成功
    Unregistered,

    /// 收到点播 INVITE（媒体层应开始推流）
    InviteReceived {
        call_id: String,
        /// 平台希望接收 RTP 的目标地址
        rtp_target_ip: String,
        rtp_target_port: u16,
        /// 从 SDP offer 中解析的 SSRC
        ssrc: String,
    },

    /// 点播已接受（200 OK 已发出）
    InviteAccepted {
        call_id: String,
        /// 回复给平台的 SDP answer
        sdp_answer: String,
    },

    /// 点播结束（BYE 已收到）
    InviteEnded { call_id: String },

    /// 收到目录查询
    CatalogQueried { sn: u32 },

    /// 收到设备信息查询
    DeviceInfoQueried { sn: u32 },

    /// 收到设备状态查询
    DeviceStatusQueried { sn: u32 },

    /// 收到录像查询
    RecordInfoQueried { sn: u32 },

    /// 收到配置下载查询
    ConfigDownloadQueried { sn: u32 },

    /// 收到预置位查询
    PresetQueryQueried { sn: u32 },
}

pub type DeviceEventHandler = Arc<
    dyn Fn(DeviceEvent) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>
        + Send
        + Sync,
>;

static EVENT_LISTENERS: LazyLock<Arc<RwLock<Vec<DeviceEventHandler>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

pub fn add_event_listener<F, Fut>(handler: F)
where
    F: Fn(DeviceEvent) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let h: DeviceEventHandler = Arc::new(move |ev| Box::pin(handler(ev)));
    EVENT_LISTENERS
        .write()
        .expect("device event listener write lock")
        .push(h);
}

pub async fn emit(event: DeviceEvent) {
    let listeners = {
        EVENT_LISTENERS
            .read()
            .expect("device event listener read lock")
            .clone()
    };
    for listener in listeners.iter() {
        if let Err(e) = listener(event.clone()).await {
            tracing::warn!(error = %e, "设备事件处理器返回错误");
        }
    }
}

#[allow(dead_code)]
pub fn clear_listeners() {
    EVENT_LISTENERS
        .write()
        .expect("device event listener write lock")
        .clear();
}
