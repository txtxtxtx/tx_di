//! 认证 HTTP API

use axum::{Json, Router, routing::{get, post}};
use axum::response::IntoResponse;
use tx_di_axum::bound::DiComp;
use admin_app::auth::app_service::AuthAppService;
use admin_proto::{LoginRequest, LoginResponse, GetUserInfoRequest, UserInfoResponse, LogoutRequest, Empty};
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/user-info", post(user_info))
        .route("/logout", post(logout))
}

/// POST /api/auth/login
async fn login(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::auth::dto::LoginCommand {
        username: req.username,
        password: req.password,
        login_ip: req.login_ip,
    };
    match auth.login(cmd).await {
        Ok(r) => ApiR::success(r),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// POST /api/auth/user-info
async fn user_info(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<GetUserInfoRequest>,
) -> impl IntoResponse {
    match auth.get_user_info(req.user_id).await {
        Ok(r) => ApiR::success(r),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// POST /api/auth/logout
async fn logout(
    DiComp(auth): DiComp<AuthAppService>,
    Json(req): Json<LogoutRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::auth::dto::LogoutCommand {
        user_id: req.user_id,
    };
    match auth.logout(cmd).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}
