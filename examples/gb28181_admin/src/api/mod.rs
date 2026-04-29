//! API 路由汇总

pub mod devices;
pub mod sessions;
pub mod sse;

use axum::{Router, routing::{get, post, delete}};

/// 构建 /api/gb28181/ 路由树
pub fn router() -> Router {
    Router::new()
        // 统计信息
        .route("/api/gb28181/stats", get(devices::stats))
        // 设备管理
        .route("/api/gb28181/devices", get(devices::list))
        .route("/api/gb28181/devices/{id}", get(devices::detail))
        .route("/api/gb28181/devices/{id}/catalog", post(devices::query_catalog))
        .route("/api/gb28181/devices/{id}/info", post(devices::query_info))
        .route("/api/gb28181/devices/{id}/status", post(devices::query_status))
        .route("/api/gb28181/devices/{id}/ptz", post(devices::ptz))
        .route("/api/gb28181/devices/{id}/teleboot", post(devices::teleboot))
        .route("/api/gb28181/devices/{id}/alarm_reset", post(devices::alarm_reset))
        // 会话管理
        .route("/api/gb28181/sessions", get(sessions::list))
        .route("/api/gb28181/sessions", post(sessions::invite))
        .route("/api/gb28181/sessions/{call_id}", delete(sessions::hangup))
        // SSE 实时事件流
        .route("/api/gb28181/events", get(sse::handler))
}
