//! 认证 HTTP API
//!
//! API 层只做 HTTP 协议适配，Session 管理和权限绑定
//! 全部由 Application 层的 `AuthSessionService` 负责。

use crate::error::ApiErr;
use admin_app::auth::app_service::AuthAppService;
use admin_app::auth::session_service::AuthSessionService;
use admin_domain::menu::model::value_object::MenuTreeNode;
use admin_proto::{Empty, LoginRequest, LoginResponse, LogoutRequest, UserInfoResponse};
use axum::Json;
use tx_common::{ApiR, ApiRes};
use tx_di_axum::Router;
use tx_di_axum::bound::DiComp;
use tx_di_sa_token::LoginIdExtractor;

/// 公开路由（无需认证）
pub fn open_router() -> Router {
    use axum::routing::post;
    Router::new().route("/api/auth/login", post(login))
}

/// 受保护路由（需要认证）
pub fn router() -> Router {
    use axum::routing::{get, post};
    Router::new()
        .route("/user_info", get(user_info))
        .route("/menus", get(user_menus))
        .route("/logout", post(logout))
}

/// POST /api/auth/login
async fn login(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<LoginRequest>,
) -> Result<ApiR<LoginResponse>, ApiErr> {
    // 登录逻辑（含 session 创建）全部在 App 层完成，API 层只转发
    let r = auth.login(req).await?;
    Ok(ApiR::success(r))
}

/// GET /api/auth/user-info
async fn user_info(
    DiComp(auth): DiComp<AuthAppService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<ApiR<UserInfoResponse>, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);
    let r = auth.get_user_info(user_id).await?;
    Ok(ApiR::success(r))
}

/// GET /api/auth/menus - 获取当前用户的菜单树
async fn user_menus(
    DiComp(auth): DiComp<AuthAppService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<ApiR<Vec<MenuTreeNode>>, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);
    let menus = auth.get_user_menus(user_id).await?;
    Ok(ApiR::success(menus))
}

/// POST /api/auth/logout
async fn logout(
    DiComp(auth): DiComp<AuthAppService>,
    DiComp(session): DiComp<AuthSessionService>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<ApiR<Empty>, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);

    // 1. 销毁 sa-token 会话
    session.logout(&login_id).await?;

    // 2. 记录登出日志
    let _ = auth.logout(LogoutRequest { user_id }).await;

    Ok(ApiRes::ok().into_typed())
}
