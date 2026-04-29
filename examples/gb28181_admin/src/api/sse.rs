//! Server-Sent Events (SSE) 实时事件推送
//!
//! 前端通过 GET /api/gb28181/events 建立 SSE 长连接，
//! 每当 GB28181 事件触发时，后端推送 JSON 事件给所有已连接的客户端。

use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;
use std::sync::LazyLock;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;
use tx_di_gb28181::Gb28181Event;

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

/// GET /api/gb28181/events — SSE 端点
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

/// 将事件序列化为 JSON 字符串
fn event_to_json(event: &Gb28181Event) -> String {
    match event {
        Gb28181Event::DeviceRegistered { device_id, remote_addr, .. } => {
            serde_json::json!({
                "type": "DeviceRegistered",
                "device_id": device_id,
                "remote_addr": remote_addr,
            })
        }
        Gb28181Event::DeviceUnregistered { device_id } => {
            serde_json::json!({ "type": "DeviceUnregistered", "device_id": device_id })
        }
        Gb28181Event::DeviceOffline { device_id } => {
            serde_json::json!({ "type": "DeviceOffline", "device_id": device_id })
        }
        Gb28181Event::DeviceOnline { device_id } => {
            serde_json::json!({ "type": "DeviceOnline", "device_id": device_id })
        }
        Gb28181Event::Keepalive { device_id, status } => {
            serde_json::json!({ "type": "Keepalive", "device_id": device_id, "status": status })
        }
        Gb28181Event::CatalogReceived { device_id, channel_count, .. } => {
            serde_json::json!({
                "type": "CatalogReceived",
                "device_id": device_id,
                "channel_count": channel_count,
            })
        }
        Gb28181Event::AlarmReceived {
            device_id, alarm_time, alarm_type, alarm_priority, alarm_description, ..
        } => {
            serde_json::json!({
                "type": "AlarmReceived",
                "device_id": device_id,
                "alarm_time": alarm_time,
                "alarm_type": alarm_type,
                "alarm_priority": alarm_priority,
                "alarm_description": alarm_description,
            })
        }
        Gb28181Event::SessionStarted { device_id, channel_id, call_id, rtp_port, .. } => {
            serde_json::json!({
                "type": "SessionStarted",
                "device_id": device_id,
                "channel_id": channel_id,
                "call_id": call_id,
                "rtp_port": rtp_port,
            })
        }
        Gb28181Event::SessionEnded { device_id, channel_id, call_id } => {
            serde_json::json!({
                "type": "SessionEnded",
                "device_id": device_id,
                "channel_id": channel_id,
                "call_id": call_id,
            })
        }
        Gb28181Event::MobilePosition { device_id, longitude, latitude, speed, .. } => {
            serde_json::json!({
                "type": "MobilePosition",
                "device_id": device_id,
                "longitude": longitude,
                "latitude": latitude,
                "speed": speed,
            })
        }
        _ => {
            serde_json::json!({ "type": "Other" })
        }
    }
    .to_string()
}
