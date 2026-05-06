//! GB28181 事件总线
//!
//! 基于 `tokio::sync::broadcast` 实现，支持多订阅者并发接收事件。
//!
//! - **回调订阅**：`Gb28181Server::on_event(|ev| async { ... })`（兼容旧 API）
//! - **通道订阅**：`event::subscribe()` 返回 `broadcast::Receiver`，适用于需要精细控制消费节奏的场景
//!
//! 所有事件发布均为 O(1) 操作（无锁、无遍历），订阅者之间互不阻塞。

use crate::device_registry::ChannelInfo;
use crate::xml::{CruiseTrack, RecordItem};
use std::future::Future;
use std::sync::OnceLock;
use tokio::sync::broadcast;

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

// ── broadcast 通道 ──────────────────────────────────────────────────────────

/// 广播通道容量（超过此数量的未消费事件将触发 Lagged 通知）
const EVENT_CHANNEL_CAPACITY: usize = 4096;

/// 全局事件广播发送器（懒初始化，线程安全）
static EVENT_TX: OnceLock<broadcast::Sender<Gb28181Event>> = OnceLock::new();

/// 获取全局事件发送器（首次调用时初始化通道）
fn event_sender() -> &'static broadcast::Sender<Gb28181Event> {
    EVENT_TX.get_or_init(|| {
        let (tx, _rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        tx
    })
}

/// 订阅事件，返回 `broadcast::Receiver`
///
/// 每次调用都会创建一个独立的订阅者，各订阅者之间互不影响。
/// 推荐在 `build()` 之前调用，以确保不遗漏早期事件。
///
/// # 示例
///
/// ```rust,ignore
/// let mut rx = event::subscribe();
/// tokio::spawn(async move {
///     loop {
///         match rx.recv().await {
///             Ok(ev) => { /* 处理事件 */ }
///             Err(broadcast::error::RecvError::Lagged(n)) => {
///                 tracing::warn!("落后 {n} 条事件");
///             }
///             Err(broadcast::error::RecvError::Closed) => break,
///         }
///     }
/// });
/// ```
pub fn subscribe() -> broadcast::Receiver<Gb28181Event> {
    event_sender().subscribe()
}

/// 注册事件监听器（回调方式，兼容旧 API）
///
/// 内部通过 `subscribe()` 获取 `Receiver`，在独立 tokio task 中驱动回调。
/// 若需要更精细的控制（如 lagged 处理、背压策略），请直接使用 `subscribe()`。
pub fn add_event_listener<F, Fut>(handler: F)
where
    F: Fn(Gb28181Event) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let mut rx = subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(e) = handler(event).await {
                        tracing::warn!(error = %e, "GB28181 事件处理器返回错误");
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        skipped = n,
                        "GB28181 事件订阅者落后，跳过了 {n} 条事件"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// 发布事件（广播给所有订阅者）
///
/// O(1) 操作：仅一次内存拷贝 + 通知所有 Receiver。
/// 无订阅者时静默忽略。
pub async fn emit(event: Gb28181Event) {
    let _ = event_sender().send(event);
}

/// 巡航轨迹详情信息（与 xml::CruiseTrack 相同）
pub type CruiseTrackInfo = CruiseTrack;
