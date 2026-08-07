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
mod health_api;

use tx_di_axum::Router;

/// API 版本前缀（阶段 E-4：路由版本化，便于未来 v2 演进与多版本共存）
///
/// 所有业务路由统一挂在 `/api/v1` 下；健康检查 `/health` 保持不带版本（基础设施探针）。
pub const API_VERSION: &str = "/api/v1";

/// 公开路由（无需登录认证）
///
/// 各模块如需添加公开接口，在此处 .merge(module::open_router()) 即可。
pub fn open_router() -> Router {
    Router::new()
        .merge(auth_api::open_router())
        .merge(file_api::open_router())
        .merge(health_api::open_router())
}

/// 注册所有受保护 HTTP 路由（需要登录认证）
///
/// `max_body_size`: 全局请求体上限（字节），用于文件上传的 Content-Length 提前拦截
pub fn router(max_body_size: u64) -> Router {
    Router::new()
        .nest(&format!("{API_VERSION}/auth"), auth_api::router())
        .nest(&format!("{API_VERSION}/user"), user_api::router())
        .nest(&format!("{API_VERSION}/role"), role_api::router())
        .nest(&format!("{API_VERSION}/menu"), menu_api::router())
        .nest(&format!("{API_VERSION}/dept"), dept_api::router())
        .nest(&format!("{API_VERSION}/config"), config_api::router())
        .nest(&format!("{API_VERSION}/dict"), dict_api::router())
        .nest(&format!("{API_VERSION}/log"), log_api::router())
        .nest(&format!("{API_VERSION}/file"), file_api::router(max_body_size))
        .nest(&format!("{API_VERSION}/monitor"), monitor_api::router())
        .nest(&format!("{API_VERSION}/job"), job_api::router())
        .nest(&format!("{API_VERSION}/tool"), tool_api::router())
}
