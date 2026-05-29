//! API 路由模块

mod auth_api;
mod user_api;
mod role_api;
mod permission_api;
mod tenant_api;
mod file_api;

use axum::{Router, routing::get};
use std::sync::Arc;
use tx_di_core::App;

/// 注册所有 API 路由
///
/// 路由结构：
/// ```text
/// /api/v1/auth/*       — 认证
/// /api/v1/users/*      — 用户管理
/// /api/v1/roles/*      — 角色管理
/// /api/v1/permissions/* — 权限管理
/// /api/v1/tenants/*    — 租户管理
/// /api/v1/files/*      — 文件服务
/// ```
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/api/v1/auth", auth_api::router(app.clone()))
        .nest("/api/v1/users", user_api::router(app.clone()))
        .nest("/api/v1/roles", role_api::router(app.clone()))
        .nest("/api/v1/permissions", permission_api::router(app.clone()))
        .nest("/api/v1/tenants", tenant_api::router(app.clone()))
        .nest("/api/v1/files", file_api::router(app.clone()))
        .route("/health", get(health_check))
}

/// 健康检查
async fn health_check() -> &'static str {
    "ok"
}
