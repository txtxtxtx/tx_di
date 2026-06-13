//! HTTP API 路由注册
//!
//! Handler 通过 DiComp<T> 从 DI 容器注入 AppService，无需手动传递 App。

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

use tx_di_axum::Router;

/// 注册所有 HTTP 路由
pub fn router() -> Router {
    Router::new()
        .nest("/api/auth", auth_api::router())
        .nest("/api/user", user_api::router())
        .nest("/api/role", role_api::router())
        .nest("/api/menu", menu_api::router())
        .nest("/api/dept", dept_api::router())
        .nest("/api/permission", permission_api::router())
        .nest("/api/config", config_api::router())
        .nest("/api/dict", dict_api::router())
        .nest("/api/log", log_api::router())
        .nest("/api/file", file_api::router())
}
