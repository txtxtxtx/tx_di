//! 认证 API
//!
//! Handler 只负责 HTTP 协议转换，业务逻辑交给领域服务。

use axum::{Json, Router, extract::State, routing::{get, post}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::AppError;
use crate::domain::user::service::UserService;
use crate::domain::user::repo::ToastyUserRepository;
use crate::domain::role::repo::ToastyRoleRepository;
use crate::domain::tenant::repo::ToastyTenantRepository;
use crate::interfaces::dto::common::ApiResponse;
use crate::interfaces::dto::auth_dto::{LoginRequest, LoginResponse, UserInfo};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/user-info", get(user_info))
        .with_state(app)
}

/// 组装 UserService（从 DI 容器提取仓储，注入领域服务）
fn build_service(app: &Arc<App>) -> UserService {
    UserService::new(
        app.inject::<ToastyUserRepository>(),
        app.inject::<ToastyRoleRepository>(),
        app.inject::<ToastyTenantRepository>(),
    )
}

async fn login(
    State(app): State<Arc<App>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let service = build_service(&app);

    // 领域服务处理认证逻辑
    let auth = service.authenticate(&req.username, &req.password).await?;

    // 构建响应（协议层的事情）
    let role_codes: Vec<String> = auth.roles.iter().map(|r| r.code.clone()).collect();
    let permissions: Vec<String> = role_codes.iter().flat_map(|r| {
        if r == "admin" { vec!["*:*:*".to_string()] } else { vec![format!("{}:read", r)] }
    }).collect();

    // TODO: 用 sa-token 生成真正的 token
    let token = format!("{}.{}.{}", auth.user.id, auth.user.username, chrono::Utc::now().timestamp());

    Ok(Json(ApiResponse::success(LoginResponse {
        token,
        user: UserInfo {
            id: auth.user.id,
            username: auth.user.username,
            nickname: auth.user.nickname,
            avatar: Some(auth.user.avatar),
            roles: role_codes,
            permissions,
            tenant_id: auth.user.tenant_id,
        },
    })))
}

async fn user_info(
    State(app): State<Arc<App>>,
) -> Result<Json<ApiResponse<UserInfo>>, AppError> {
    let service = build_service(&app);

    // TODO: 从 sa-token 获取当前用户 ID，硬编码 1 仅供调试
    let (user, roles) = service.get_user_with_roles(1).await?;

    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
    let permissions: Vec<String> = role_codes.iter().flat_map(|r| {
        if r == "admin" { vec!["*:*:*".to_string()] } else { vec![format!("{}:read", r)] }
    }).collect();

    Ok(Json(ApiResponse::success(UserInfo {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        avatar: Some(user.avatar),
        roles: role_codes,
        permissions,
        tenant_id: user.tenant_id,
    })))
}
