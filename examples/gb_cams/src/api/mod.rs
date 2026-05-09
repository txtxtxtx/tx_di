pub mod devices;
pub mod events;

use axum::{routing::{get, post, delete}, Router};

/// 构建 /api/gb_cams/ 路由树
pub fn router() -> Router {
    Router::new()
        .route("/api/gb28181/stats", get(devices::stats))
        .route("/api/gb28181/devices", get(devices::list))
        .route("/api/gb28181/devices", post(devices::create))
        .route("/api/gb28181/devices/generate", post(devices::generate))
        .route("/api/gb28181/devices/{id}", get(devices::detail))
        .route("/api/gb28181/devices/{id}", delete(devices::remove))
        .route("/api/gb28181/events", get(events::handler))
}
