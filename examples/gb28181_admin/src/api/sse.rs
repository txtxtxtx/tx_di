//! Server-Sent Events (SSE) 实时事件推送
//!
//! 前端通过 GET /api/v1/gb28181/events 建立 SSE 长连接，
//! 每当 GB28181 事件触发时，后端推送 JSON 事件给所有已连接的客户端。
//!
//! 支持全部 30 种 Gb28181Event 变体（含 GB28181-2022 新增事件）。

use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;
use std::sync::LazyLock;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;
use tx_gb28181::Gb28181Event;

/// 全局广播通道（容量 256 条消息）
static TX: LazyLock<broadcast::Sender<String>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

/// 将 GB28181 事件序列化后广播给所有 SSE 客户端
pub fn broadcast_event(event: Gb28181Event) {
    let payload = event_to_json(&event);
    // 忽略"无接收者"错误（无客户端连接时正常）
    let _ = TX.send(payload);
}

/// GET /api/v1/gb28181/events — SSE 端点
pub async fn handler() -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let rx = TX.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            result.ok().map(|data| {
                Ok(Event::default().data(data))
            })
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// 将事件序列化为 JSON 字符串（覆盖全部 30 种变体）
fn event_to_json(event: &Gb28181Event) -> String {
    use serde_json::json;
    let v = match event {
        // ── 设备注册管理 ──────────────────────────────────────────────────────
        Gb28181Event::DeviceRegistered { device_id, contact, remote_addr } => json!({
            "type": "DeviceRegistered",
            "device_id": device_id,
            "contact": contact,
            "remote_addr": remote_addr,
        }),
        Gb28181Event::DeviceUnregistered { device_id } => json!({
            "type": "DeviceUnregistered",
            "device_id": device_id,
        }),
        Gb28181Event::DeviceOffline { device_id } => json!({
            "type": "DeviceOffline",
            "device_id": device_id,
        }),
        Gb28181Event::DeviceOnline { device_id } => json!({
            "type": "DeviceOnline",
            "device_id": device_id,
        }),
        Gb28181Event::Keepalive { device_id, status } => json!({
            "type": "Keepalive",
            "device_id": device_id,
            "status": status,
        }),

        // ── 设备查询响应 ──────────────────────────────────────────────────────
        Gb28181Event::CatalogReceived { device_id, channel_count, .. } => json!({
            "type": "CatalogReceived",
            "device_id": device_id,
            "channel_count": channel_count,
        }),
        Gb28181Event::DeviceInfoReceived { device_id, manufacturer, model, firmware, channel_num } => json!({
            "type": "DeviceInfoReceived",
            "device_id": device_id,
            "manufacturer": manufacturer,
            "model": model,
            "firmware": firmware,
            "channel_num": channel_num,
        }),
        Gb28181Event::DeviceStatusReceived { device_id, online, status, encode, record } => json!({
            "type": "DeviceStatusReceived",
            "device_id": device_id,
            "online": online,
            "status": status,
            "encode": encode,
            "record": record,
        }),
        Gb28181Event::RecordInfoReceived { device_id, sum_num, items } => json!({
            "type": "RecordInfoReceived",
            "device_id": device_id,
            "sum_num": sum_num,
            "count": items.len(),
        }),

        // ── 媒体会话 ──────────────────────────────────────────────────────────
        Gb28181Event::SessionStarted { device_id, channel_id, call_id, rtp_port, ssrc } => json!({
            "type": "SessionStarted",
            "device_id": device_id,
            "channel_id": channel_id,
            "call_id": call_id,
            "rtp_port": rtp_port,
            "ssrc": ssrc,
        }),
        Gb28181Event::SessionEnded { device_id, channel_id, call_id } => json!({
            "type": "SessionEnded",
            "device_id": device_id,
            "channel_id": channel_id,
            "call_id": call_id,
        }),
        Gb28181Event::MediaStatusNotify { device_id, notify_type } => json!({
            "type": "MediaStatusNotify",
            "device_id": device_id,
            "notify_type": notify_type,
        }),

        // ── 报警 ──────────────────────────────────────────────────────────────
        Gb28181Event::AlarmReceived {
            device_id, alarm_time, alarm_type, alarm_priority,
            alarm_description, longitude, latitude,
        } => json!({
            "type": "AlarmReceived",
            "device_id": device_id,
            "alarm_time": alarm_time,
            "alarm_type": alarm_type,
            "alarm_priority": alarm_priority,
            "alarm_description": alarm_description,
            "longitude": longitude,
            "latitude": latitude,
        }),

        // ── 位置 ──────────────────────────────────────────────────────────────
        Gb28181Event::MobilePosition { device_id, longitude, latitude, speed, direction } => json!({
            "type": "MobilePosition",
            "device_id": device_id,
            "longitude": longitude,
            "latitude": latitude,
            "speed": speed,
            "direction": direction,
        }),
        Gb28181Event::MobilePositionQueryResult {
            device_id, longitude, latitude, speed, direction, timestamp,
        } => json!({
            "type": "MobilePositionQueryResult",
            "device_id": device_id,
            "longitude": longitude,
            "latitude": latitude,
            "speed": speed,
            "direction": direction,
            "timestamp": timestamp,
        }),

        // ── 网络校时 ──────────────────────────────────────────────────────────
        Gb28181Event::TimeSyncResult { device_id, device_time, time_diff_secs } => json!({
            "type": "TimeSyncResult",
            "device_id": device_id,
            "device_time": device_time,
            "time_diff_secs": time_diff_secs,
        }),

        // ── 配置/预置位/巡航 ──────────────────────────────────────────────────
        Gb28181Event::ConfigDownloaded { device_id, config_type, items } => json!({
            "type": "ConfigDownloaded",
            "device_id": device_id,
            "config_type": config_type,
            "items": items.iter().map(|(k,v)| json!({"name": k, "value": v})).collect::<Vec<_>>(),
        }),
        Gb28181Event::PresetListReceived { device_id, channel_id, presets } => json!({
            "type": "PresetListReceived",
            "device_id": device_id,
            "channel_id": channel_id,
            "presets": presets.iter().map(|(id,name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        }),
        Gb28181Event::CruiseListReceived { device_id, channel_id, cruises } => json!({
            "type": "CruiseListReceived",
            "device_id": device_id,
            "channel_id": channel_id,
            "cruises": cruises.iter().map(|(id,name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        }),
        Gb28181Event::CruiseTrackReceived { device_id, channel_id, tracks } => json!({
            "type": "CruiseTrackReceived",
            "device_id": device_id,
            "channel_id": channel_id,
            "track_count": tracks.len(),
        }),
        Gb28181Event::PtzPreciseStatusReceived {
            device_id, channel_id,
            pan_position, tilt_position, zoom_position,
            focus_position, iris_position,
        } => json!({
            "type": "PtzPreciseStatusReceived",
            "device_id": device_id,
            "channel_id": channel_id,
            "pan": pan_position,
            "tilt": tilt_position,
            "zoom": zoom_position,
            "focus": focus_position,
            "iris": iris_position,
        }),
        Gb28181Event::GuardInfoReceived { device_id, guard_id, preset_index } => json!({
            "type": "GuardInfoReceived",
            "device_id": device_id,
            "guard_id": guard_id,
            "preset_index": preset_index,
        }),

        // ── 图像抓拍 ──────────────────────────────────────────────────────────
        Gb28181Event::SnapshotTaken { device_id, channel_id, image_url } => json!({
            "type": "SnapshotTaken",
            "device_id": device_id,
            "channel_id": channel_id,
            "image_url": image_url,
        }),

        // ── 语音广播/对讲 ─────────────────────────────────────────────────────
        Gb28181Event::BroadcastInviteReceived { device_id, source_id } => json!({
            "type": "BroadcastInviteReceived",
            "device_id": device_id,
            "source_id": source_id,
        }),
        Gb28181Event::BroadcastSessionStarted { device_id, audio_port } => json!({
            "type": "BroadcastSessionStarted",
            "device_id": device_id,
            "audio_port": audio_port,
        }),
        Gb28181Event::BroadcastSessionEnded { device_id } => json!({
            "type": "BroadcastSessionEnded",
            "device_id": device_id,
        }),
        Gb28181Event::AudioTalkbackStarted { device_id, channel_id, call_id, device_ip, device_port } => json!({
            "type": "AudioTalkbackStarted",
            "device_id": device_id,
            "channel_id": channel_id,
            "call_id": call_id,
            "device_ip": device_ip,
            "device_port": device_port,
        }),
        Gb28181Event::AudioTalkbackEnded { device_id, call_id } => json!({
            "type": "AudioTalkbackEnded",
            "device_id": device_id,
            "call_id": call_id,
        }),

        // ── 设备控制结果 ──────────────────────────────────────────────────────
        Gb28181Event::RecordControlResult { device_id, channel_id, result } => json!({
            "type": "RecordControlResult",
            "device_id": device_id,
            "channel_id": channel_id,
            "result": result,
        }),
        Gb28181Event::ConfigPushResult { device_id, config_type, result } => json!({
            "type": "ConfigPushResult",
            "device_id": device_id,
            "config_type": config_type,
            "result": result,
        }),
        Gb28181Event::PtLockResult { device_id, channel_id, locked, result } => json!({
            "type": "PtLockResult",
            "device_id": device_id,
            "channel_id": channel_id,
            "locked": locked,
            "result": result,
        }),

        // ── 级联管理 ──────────────────────────────────────────────────────────
        Gb28181Event::UpperPlatformRegistered { platform_id, contact } => json!({
            "type": "UpperPlatformRegistered",
            "platform_id": platform_id,
            "contact": contact,
        }),
        Gb28181Event::UpperPlatformUnregistered { platform_id } => json!({
            "type": "UpperPlatformUnregistered",
            "platform_id": platform_id,
        }),
    };
    v.to_string()
}
