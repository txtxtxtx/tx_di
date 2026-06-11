//! 认证 HTTP API
//!
//! 使用 admin_proto 生成的 LoginRequest/LoginResponse 等 DTO，
//! HTTP 协议层仅负责 JSON ↔ Proto DTO 转换，业务逻辑委托给 admin_app。

use axum::{Json, Router, extract::State, routing::{get, post}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{LoginRequest, LoginResponse, UserInfoResponse, LogoutRequest, Empty};
use crate::interfaces::dto::ApiResponse;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/user-info", get(user_info))
        .route("/logout", post(logout))
        .with_state(app)
}

/// POST /api/auth/login
///
/// 接收 JSON 格式的 LoginRequest（proto 生成），返回 LoginResponse
async fn login(
    State(_app): State<Arc<App>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, tx_error::AppError> {
    // TODO: 调用 AuthAppService::login，目前为占位实现
    let resp = LoginResponse {
        user_id: 1,
        username: req.username.clone(),
        nickname: "Admin".into(),
        tenant_id: 0,
        role_ids: vec![],
        permissions: vec![],
        dept_ids: vec![],
        token: "placeholder-token".into(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/auth/user-info?user_id=1
async fn user_info(
    State(_app): State<Arc<App>>,
) -> Result<Json<ApiResponse<UserInfoResponse>>, tx_error::AppError> {
    // TODO: 调用 AuthAppService::get_user_info
    let resp = UserInfoResponse {
        user_id: 1,
        username: "admin".into(),
        nickname: "Admin".into(),
        email: Some("admin@example.com".into()),
        mobile: None,
        avatar: None,
        roles: vec!["admin".into()],
        permissions: vec!["*:*:*".into()],
        tenant_id: 0,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// POST /api/auth/logout
async fn logout(
    State(_app): State<Arc<App>>,
    Json(_req): Json<LogoutRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 AuthAppService::logout
    Ok(Json(ApiResponse::success(Empty {})))
}
