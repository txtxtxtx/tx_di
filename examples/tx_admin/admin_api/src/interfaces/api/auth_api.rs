//! 认证 HTTP API

use axum::Json;
use tx_di_axum::{Router, R};
use tx_di_axum::aide::axum::routing::{get, post};
use tx_di_axum::bound::DiComp;
use admin_app::auth::app_service::AuthAppService;
use admin_proto::{LoginRequest, GetUserInfoRequest, LogoutRequest, Empty};
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        .api_route("/login", post(login))
        .api_route("/user-info", post(user_info))
        .api_route("/logout", post(logout))
}

/// POST /api/auth/login
async fn login(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<LoginRequest>,
) -> R<admin_app::auth::dto::LoginResponse> {
    let cmd = admin_app::auth::dto::LoginCommand {
        username: req.username,
        password: req.password,
        login_ip: req.login_ip,
    };
    match auth.login(cmd).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/auth/user-info
async fn user_info(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<GetUserInfoRequest>,
) -> R<admin_app::auth::dto::UserInfoResponse> {
    match auth.get_user_info(req.user_id).await {
        Ok(r) => R(ApiR::success(r)),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// POST /api/auth/logout
async fn logout(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<LogoutRequest>,
) -> R<Empty> {
    let cmd = admin_app::auth::dto::LogoutCommand {
        user_id: req.user_id,
    };
    match auth.logout(cmd).await {
        Ok(()) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}
