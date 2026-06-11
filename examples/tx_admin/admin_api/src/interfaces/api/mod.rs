//! HTTP API 路由注册
//!
//! 所有 HTTP handler 使用 admin_proto 生成的 DTO，
//! 外层用 ApiResponse 包装为统一 JSON 响应格式。

mod auth_api;

use axum::Router;
use std::sync::Arc;
use tx_di_core::App;

/// 注册所有 HTTP 路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // ── 认证 ──
        .nest("/api/auth", auth_api::router(app))
}
