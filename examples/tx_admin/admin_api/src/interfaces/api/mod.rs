//! HTTP API 路由注册
//!
//! 所有 HTTP handler 使用 admin_proto 生成的 DTO，
//! 外层用 ApiResponse 包装为统一 JSON 响应格式。

mod auth_api;
mod user_api;
mod role_api;
mod menu_api;
mod dept_api;
mod permission_api;
mod config_api;
mod dict_api;
mod log_api;
mod file_api;

use axum::Router;
use std::sync::Arc;
use tx_di_core::App;

/// 注册所有 HTTP 路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // ── 认证 ──
        .nest("/api/auth", auth_api::router(app.clone()))
        // ── 用户 ──
        .nest("/api/user", user_api::router(app.clone()))
        // ── 角色 ──
        .nest("/api/role", role_api::router(app.clone()))
        // ── 菜单 ──
        .nest("/api/menu", menu_api::router(app.clone()))
        // ── 部门 ──
        .nest("/api/dept", dept_api::router(app.clone()))
        // ── 权限 ──
        .nest("/api/permission", permission_api::router(app.clone()))
        // ── 配置 ──
        .nest("/api/config", config_api::router(app.clone()))
        // ── 字典 ──
        .nest("/api/dict", dict_api::router(app.clone()))
        // ── 日志 ──
        .nest("/api/log", log_api::router(app.clone()))
        // ── 文件 ──
        .nest("/api/file", file_api::router(app))
}
