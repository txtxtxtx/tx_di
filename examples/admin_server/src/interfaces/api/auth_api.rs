//! 认证 API

use axum::{Json, Router, extract::State, routing::{get, post}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::error::AdminError;
use crate::domain::user::UserRepository;
use crate::domain::role::RoleRepository;
use crate::domain::tenant::TenantRepository;
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

async fn login(
    State(app): State<Arc<App>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AdminError> {
    let user_repo = app.inject::<ToastyUserRepository>();
    let role_repo = app.inject::<ToastyRoleRepository>();
    let tenant_repo = app.inject::<ToastyTenantRepository>();

    let user = user_repo.find_by_username(&req.username).await?.ok_or(AdminError::BadCredentials)?;
    if !bcrypt::verify(&req.password, &user.password_hash).unwrap_or(false) { return Err(AdminError::BadCredentials); }
    if !user.is_active() { return Err(AdminError::UserDisabled); }

    let tenant = tenant_repo.find_by_id(user.tenant_id).await?.ok_or(AdminError::TenantDisabled)?;
    if !tenant.is_active() { return Err(AdminError::TenantDisabled); }

    let roles = role_repo.find_by_tenant(user.tenant_id).await.unwrap_or_default();
    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
    let permissions: Vec<String> = role_codes.iter().flat_map(|r| if r == "admin" { vec!["*:*:*".to_string()] } else { vec![format!("{}:read", r)] }).collect();

    let token = format!("{}.{}.{}", user.id, user.username, chrono::Utc::now().timestamp());
    Ok(Json(ApiResponse::success(LoginResponse { token, user: UserInfo { id: user.id, username: user.username, nickname: user.nickname, avatar: user.avatar, roles: role_codes, permissions, tenant_id: user.tenant_id } })))
}

async fn user_info(
    State(app): State<Arc<App>>,
) -> Result<Json<ApiResponse<UserInfo>>, AdminError> {
    let user_repo = app.inject::<ToastyUserRepository>();
    let user = user_repo.find_by_id(1).await?.ok_or(AdminError::UserNotFound("1".to_string()))?;
    Ok(Json(ApiResponse::success(UserInfo { id: user.id, username: user.username, nickname: user.nickname, avatar: user.avatar, roles: vec!["admin".to_string()], permissions: vec!["*:*:*".to_string()], tenant_id: user.tenant_id })))
}
