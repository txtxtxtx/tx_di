pub mod devices;
pub mod events;

use axum::{routing::{get, post, delete}, Router};

/// 构建 /api/gb_cams/ 路由树
pub fn router() -> Router {
    Router::new()
        .route("/api/gb_cams/stats", get(devices::stats))
        .route("/api/gb_cams/devices", get(devices::list))
        .route("/api/gb_cams/devices", post(devices::create))
        .route("/api/gb_cams/devices/generate", post(devices::generate))
        .route("/api/gb_cams/devices/{id}", get(devices::detail))
        .route("/api/gb_cams/devices/{id}", delete(devices::remove))
        .route("/api/gb_cams/events", get(events::handler))
}
