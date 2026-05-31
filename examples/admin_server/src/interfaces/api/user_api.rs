//! 用户管理 API
//!
//! Handler 只负责 HTTP 协议转换，业务逻辑交给领域服务。

use axum::{Json, Router, extract::{Path, Query, State}, routing::{delete, get, post, put}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::error::{AdminError, AdminErr};
use crate::domain::user::UserRepository;
use crate::domain::user::service::UserService;
use crate::domain::user::repo::ToastyUserRepository;
use crate::domain::role::repo::ToastyRoleRepository;
use crate::domain::tenant::repo::ToastyTenantRepository;
use crate::interfaces::dto::common::{ApiResponse, PageQuery, PageResponse};
use crate::interfaces::dto::user_dto::{UserDto, CreateUserRequest, UpdateUserRequest};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .with_state(app)
}

fn build_service(app: &Arc<App>) -> UserService {
    UserService::new(
        app.inject::<ToastyUserRepository>(),
        app.inject::<ToastyRoleRepository>(),
        app.inject::<ToastyTenantRepository>(),
    )
}

async fn list_users(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiResponse<PageResponse<UserDto>>>, AdminError> {
    let repo = app.inject::<ToastyUserRepository>();
    let (users, total) = repo.find_page(1, query.keyword.as_deref(), None, query.page, query.page_size).await?;
    let dtos: Vec<UserDto> = users.iter().map(UserDto::from).collect();
    Ok(Json(ApiResponse::success(PageResponse::new(dtos, total, query.page, query.page_size))))
}

async fn get_user(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<UserDto>>, AdminError> {
    let repo = app.inject::<ToastyUserRepository>();
    let user = repo.find_by_id(id).await?.ok_or(AdminError::with_context(AdminErr::UserNotFound, id.to_string()))?;
    Ok(Json(ApiResponse::success(UserDto::from(&user))))
}

async fn create_user(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserDto>>, AdminError> {
    let service = build_service(&app);
    // TODO: tenant_id 应从 sa-token 获取
    let mut user = service.register(1, &req.username, &req.password, &req.nickname).await?;
    user.email = req.email.unwrap_or_default();
    user.mobile = req.mobile.unwrap_or_default();
    // 注册后再更新补充字段
    service.user_repo.save(&user).await?;
    Ok(Json(ApiResponse::success(UserDto::from(&user))))
}

async fn update_user(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserDto>>, AdminError> {
    let repo = app.inject::<ToastyUserRepository>();
    let mut user = repo.find_by_id(id).await?.ok_or(AdminError::with_context(AdminErr::UserNotFound, id.to_string()))?;
    if let Some(n) = req.nickname { user.nickname = n; }
    if let Some(e) = req.email { user.email = e; }
    if let Some(m) = req.mobile { user.mobile = m; }
    repo.save(&user).await?;
    Ok(Json(ApiResponse::success(UserDto::from(&user))))
}

async fn delete_user(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let repo = app.inject::<ToastyUserRepository>();
    repo.find_by_id(id).await?.ok_or(AdminError::with_context(AdminErr::UserNotFound, id.to_string()))?;
    repo.delete(id).await?;
    Ok(Json(ApiResponse::<()>::ok()))
}
