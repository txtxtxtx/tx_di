//! 认证 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{LoginRequest, LoginResponse, GetUserInfoRequest, UserInfoResponse, LogoutRequest, Empty};
use crate::services;
use tx_common::{ApiR, ApiRes};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/user-info", post(user_info))
        .route("/logout", post(logout))
        .with_state(app)
}

/// POST /api/auth/login
async fn login(
    State(_app): State<Arc<App>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::auth::dto::LoginCommand {
        username: req.username,
        password: req.password,
        login_ip: req.login_ip,
    };
    match services::get().auth.login(cmd).await {
        Ok(r) => ApiR::success(LoginResponse {
            user_id: r.user_id,
            username: r.username,
            nickname: r.nickname,
            tenant_id: r.tenant_id.into_inner() as i64,
            role_ids: r.role_ids,
            permissions: r.permissions,
            dept_ids: r.dept_ids,
            token: String::new(), // token 由中间件生成
        }),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// POST /api/auth/user-info
async fn user_info(
    State(_app): State<Arc<App>>,
    Json(req): Json<GetUserInfoRequest>,
) -> impl IntoResponse {
    match services::get().auth.get_user_info(req.user_id).await {
        Ok(r) => ApiR::success(UserInfoResponse {
            user_id: r.user_id,
            username: r.username,
            nickname: r.nickname,
            email: r.email,
            mobile: r.mobile,
            avatar: r.avatar,
            roles: r.roles,
            permissions: r.permissions,
            tenant_id: 0, // auth UserInfoResponse 暂无 tenant_id
        }),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// POST /api/auth/logout
async fn logout(
    State(_app): State<Arc<App>>,
    Json(req): Json<LogoutRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::auth::dto::LogoutCommand { user_id: req.user_id };
    match services::get().auth.logout(cmd).await {
        Ok(()) => ApiR::success(Empty {}),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
