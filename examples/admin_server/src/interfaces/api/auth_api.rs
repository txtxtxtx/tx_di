//! 认证 API
//!
//! 提供登录、登出、获取用户信息等接口。

use axum::{Json, Router, extract::State, routing::post, routing::get};
use std::sync::Arc;
use tx_di_core::App;

use crate::application::auth::{AuthService, LoginRequest};
use crate::interfaces::dto::ApiResponse;

/// 认证路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/user-info", get(user_info))
        .with_state(app)
}

/// 登录请求体
#[derive(Debug, serde::Deserialize)]
struct LoginBody {
    username: String,
    password: String,
    #[serde(default)]
    tenant_id: Option<String>,
}

/// 用户登录
async fn login(
    State(app): State<Arc<App>>,
    Json(body): Json<LoginBody>,
) -> Json<ApiResponse<crate::application::auth::LoginResponse>> {
    let service = app.inject::<AuthService>();

    match service
        .login(LoginRequest {
            username: body.username,
            password: body.password,
            tenant_id: body.tenant_id,
        })
        .await
    {
        Ok(resp) => Json(ApiResponse::success(resp)),
        Err(e) => Json(ApiResponse::error(401, e.to_string())),
    }
}

/// 用户登出
async fn logout() -> Json<ApiResponse<()>> {
    // Token 失效由客户端处理，服务端可扩展为黑名单机制
    Json(ApiResponse::ok())
}

/// 获取当前用户信息
async fn user_info(
    State(app): State<Arc<App>>,
) -> Json<ApiResponse<crate::application::auth::UserInfo>> {
    let service = app.inject::<AuthService>();

    // 实际项目应从 Token 中解析 user_id
    // 这里固定返回管理员信息
    match service.get_user_info("u-admin-001").await {
        Ok(info) => Json(ApiResponse::success(info)),
        Err(e) => Json(ApiResponse::error(401, e.to_string())),
    }
}
