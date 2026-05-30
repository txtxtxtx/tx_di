//! API 路由模块

mod auth_api;
mod user_api;
mod role_api;
mod tenant_api;

use axum::{Router, routing::get};
use std::sync::Arc;
use tx_di_core::App;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/api/v1/auth", auth_api::router(app.clone()))
        .nest("/api/v1/users", user_api::router(app.clone()))
        .nest("/api/v1/roles", role_api::router(app.clone()))
        .nest("/api/v1/tenants", tenant_api::router(app.clone()))
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str { "ok" }
