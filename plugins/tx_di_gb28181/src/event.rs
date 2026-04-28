//! GB28181 事件总线
//!
//! 提供类型安全的事件订阅/发布机制，上层业务通过 `Gb28181Server::on_event()` 订阅。

use crate::device_registry::ChannelInfo;
use crate::xml::{CruiseTrack, RecordItem};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, RwLock};

/// GB28181 事件类型（GB28181-2022 完整版）
#[derive(Debug, Clone)]
pub enum Gb28181Event {
    // ── 设备注册管理 ─────────────────────────────────────────────────────────

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

    // ── 设备查询响应 ─────────────────────────────────────────────────────────

    /// 收到目录响应（通道列表）
    CatalogReceived {
        device_id: String,
        channel_count: usize,
        channels: Vec<ChannelInfo>,
    },

    /// 收到设备信息响应
    DeviceInfoReceived {
        device_id: String,
        manufacturer: String,
        model: String,
        firmware: String,
        channel_num: u32,
    },

    /// 收到设备状态响应
    DeviceStatusReceived {
        device_id: String,
        online: String,
        status: String,
        encode: String,
        record: String,
    },

    /// 收到录像文件列表
    RecordInfoReceived {
        device_id: String,
        sum_num: u32,
        items: Vec<RecordItem>,
    },

    // ── 媒体会话 ─────────────────────────────────────────────────────────────

    /// 点播/回放会话建立（200 OK + ACK 后）
    SessionStarted {
        device_id: String,
        channel_id: String,
        call_id: String,
        rtp_port: u16,
        ssrc: String,
    },

    /// 点播/回放会话结束
    SessionEnded {
        device_id: String,
        channel_id: String,
        call_id: String,
    },

    /// 媒体状态通知（设备推流结束等）
    MediaStatusNotify {
        device_id: String,
        notify_type: String,  // 121=媒体流发送结束
    },

    // ── 报警 ─────────────────────────────────────────────────────────────────

    /// 收到设备报警通知
    AlarmReceived {
        device_id: String,
        alarm_time: String,
        alarm_type: String,
        alarm_priority: u8,
        alarm_description: String,
        longitude: Option<f64>,
        latitude: Option<f64>,
    },

    // ── 位置 ─────────────────────────────────────────────────────────────────

    /// 移动设备位置上报
    MobilePosition {
        device_id: String,
        longitude: f64,
        latitude: f64,
        speed: Option<f64>,
        direction: Option<f64>,
    },

    // ── 网络校时 ─────────────────────────────────────────────────────────────

    /// 收到设备校时响应
    ///
    /// 可用于分析设备时间偏差
    TimeSyncResult {
        device_id: String,
        device_time: String,
        /// 时间差（设备 - 本地，秒）
        time_diff_secs: f64,
    },

    // ── 配置/预置位查询 ───────────────────────────────────────────────────────

    /// 收到设备配置查询响应
    ConfigDownloaded {
        device_id: String,
        config_type: String,
        items: Vec<(String, String)>, // (name, value)
    },

    /// 收到预置位列表响应
    PresetListReceived {
        device_id: String,
        channel_id: String,
        presets: Vec<(String, String)>, // (preset_id, name)
    },

    /// 收到巡航轨迹列表响应
    CruiseListReceived {
        device_id: String,
        channel_id: String,
        cruises: Vec<(String, String)>, // (cruise_id, name)
    },

    /// 收到巡航轨迹详情响应（每个巡航轨迹的详细信息）
    ///
    /// GB28181-2022 A.2.4.12：巡航轨迹查询响应
    CruiseTrackReceived {
        device_id: String,
        channel_id: String,
        /// 巡航轨迹列表
        tracks: Vec<CruiseTrackInfo>,
    },

    /// 收到 PTZ 精准状态响应
    ///
    /// GB28181-2022 A.2.4.13（2022 新增）：PTZ 精准状态查询响应
    PtzPreciseStatusReceived {
        device_id: String,
        channel_id: String,
        pan_position: u16,
        tilt_position: u16,
        zoom_position: u16,
        focus_position: Option<u16>,
        iris_position: Option<u16>,
    },

    /// 收到看守位信息响应
    ///
    /// GB28181-2022 A.2.4.11：看守位信息查询响应
    GuardInfoReceived {
        device_id: String,
        guard_id: u8,
        preset_index: u8,
    },

    // ── 图像抓拍 ─────────────────────────────────────────────────────────────

    /// 设备抓拍完成（图片已就绪）
    SnapshotTaken {
        device_id: String,
        channel_id: String,
        /// 抓拍图片 URL（从设备 SDP 解析）
        image_url: String,
    },

    // ── 语音广播/对讲 ───────────────────────────────────────────────────────

    /// 收到设备发起的语音广播邀请
    ///
    /// 平台收到设备发来的广播 MESSAGE，需要确认是否接受
    /// 并决定接受时返回音频接收端口
    BroadcastInviteReceived {
        device_id: String,
        source_id: String,
    },

    /// 语音广播会话开始
    BroadcastSessionStarted {
        device_id: String,
        /// 设备推送音频的端口（平台监听端口）
        audio_port: u16,
    },

    /// 语音广播会话结束
    BroadcastSessionEnded {
        device_id: String,
    },

    /// 对讲音频会话建立
    AudioTalkbackStarted {
        device_id: String,
        channel_id: String,
        call_id: String,
        /// 设备发送音频的 IP 和端口
        device_ip: String,
        device_port: u16,
    },

    /// 对讲音频会话结束
    AudioTalkbackEnded {
        device_id: String,
        call_id: String,
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

/// 巡航轨迹详情信息（与 xml::CruiseTrack 相同）
pub type CruiseTrackInfo = CruiseTrack;

/// 清空所有监听器（测试用）
#[allow(dead_code)]
pub fn clear_listeners() {
    EVENT_LISTENERS
        .write()
        .expect("event listener write lock")
        .clear();
}
