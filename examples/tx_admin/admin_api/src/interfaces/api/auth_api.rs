//! 认证 HTTP API
//!
//! API 层只做 HTTP 协议适配，Session 管理和权限绑定
//! 全部由 Application 层的 `AuthSessionService` 负责。

use crate::error::ApiErr;
use admin_app::auth::app_service::AuthAppService;
use admin_app::auth::session_service::AuthSessionService;
use admin_domain::menu::model::value_object::MenuTreeNode;
use admin_proto::{Empty, LoginRequest, LogoutRequest, UserInfoResponse};
use axum::Json;
use axum::http::HeaderValue;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use tx_common::{ApiR, ApiRes};
use tx_di_axum::Router;
use tx_di_axum::bound::DiComp;
use tx_di_sa_token::{LoginIdExtractor, SaTokenConf};

/// 公开路由（无需认证）
///
/// 登录接口应用限流（阶段 E-1）：按客户端 IP 限速（默认 5 次/分钟，突发 5），
/// 防御暴力破解/撞库。IP 来源：直连时取 ConnectInfo；反向代理后需可信
/// `X-Forwarded-For`（见 SmartIpKeyExtractor 说明），否则所有请求共享代理 IP 额度。
pub fn open_router() -> Router {
    use axum::routing::post;
    use tower_governor::GovernorLayer;
    use tower_governor::governor::GovernorConfigBuilder;

    // 登录限流：每 12 秒补充 1 个配额（≈5 次/分钟），桶容量 5 允许突发
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(12)
            .burst_size(5)
            .use_headers() // 响应携带 x-ratelimit-limit/remaining 等
            .finish()
            .unwrap(),
    );

    Router::new()
        .route("/api/v1/auth/login", post(login))
        .layer(GovernorLayer::new(governor_conf))
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
///
/// 阶段 E-2（token 存储加固）：登录成功后将 token 写入 **HttpOnly Cookie**
/// （`SameSite=Lax`，防 XSS 窃取）。前端改用 `withCredentials` 携带 Cookie，
/// 不再把 token 存 localStorage / 注入 Authorization 头。
async fn login(
    DiComp(auth): DiComp<AuthAppService>,
    DiComp(sa_conf): DiComp<SaTokenConf>,
    Json(req): Json<LoginRequest>,
) -> Result<Response, ApiErr> {
    // 登录逻辑（含 session 创建）全部在 App 层完成，API 层只转发
    let r = auth.login(req).await?;
    let token = r.token.clone();

    let mut resp = ApiR::success(r).into_response();

    // HttpOnly Cookie：token_name 与 sa-token 读取键一致，超时与会话一致
    let cookie = format!(
        "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
        sa_conf.token_name, token, sa_conf.timeout
    );
    if let Ok(v) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(SET_COOKIE, v);
    }
    Ok(resp)
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
    DiComp(sa_conf): DiComp<SaTokenConf>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> Result<Response, ApiErr> {
    let user_id: u64 = login_id.parse().unwrap_or(0);

    // 1. 销毁 sa-token 会话
    session.logout(&login_id).await?;

    // 2. 记录登出日志
    let _ = auth.logout(LogoutRequest { user_id }).await;

    // 3. 清除 HttpOnly Cookie（Max-Age=0）
    let resp: ApiR<Empty> = ApiRes::ok().into_typed();
    let mut resp = resp.into_response();
    let cookie = format!(
        "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0",
        sa_conf.token_name
    );
    if let Ok(v) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(SET_COOKIE, v);
    }
    Ok(resp)
}
