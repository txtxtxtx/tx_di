//! HTTP API 路由注册
//!
//! Handler 通过 DiComp<T> 从 DI 容器注入 AppService，无需手动传递 App。

pub mod auth_api;
mod user_api;
mod role_api;
mod menu_api;
mod dept_api;
mod config_api;
mod dict_api;
mod log_api;
mod file_api;
pub mod monitor_api;
mod tool_api;
mod job_api;

use tx_di_axum::Router;

/// 公开路由（无需登录认证）
///
/// 各模块如需添加公开接口，在此处 .merge(module::open_router()) 即可。
pub fn open_router() -> Router {
    Router::new()
        .merge(auth_api::open_router())
        .merge(file_api::open_router())
}

/// 注册所有受保护 HTTP 路由（需要登录认证）
///
/// `max_body_size`: 全局请求体上限（字节），用于文件上传的 Content-Length 提前拦截
pub fn router(max_body_size: u64) -> Router {
    Router::new()
        .nest("/api/auth", auth_api::router())
        .nest("/api/user", user_api::router())
        .nest("/api/role", role_api::router())
        .nest("/api/menu", menu_api::router())
        .nest("/api/dept", dept_api::router())
        .nest("/api/config", config_api::router())
        .nest("/api/dict", dict_api::router())
        .nest("/api/log", log_api::router())
        .nest("/api/file", file_api::router(max_body_size))
        .nest("/api/monitor", monitor_api::router())
        .nest("/api/job", job_api::router())
        .nest("/api/tool", tool_api::router())
}
