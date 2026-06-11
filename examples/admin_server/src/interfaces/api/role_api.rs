//! 角色管理 API

use axum::{Json, Router, extract::{Path, Query, State}, routing::{delete, get, post, put}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::{AppError, AdminErr};
use crate::domain::role::RoleRepository;
use crate::domain::role::repo::ToastyRoleRepository;
use tx_common::{ApiR, ApiRes, Page};
use crate::interfaces::dto::common::PageQuery;
use crate::interfaces::dto::role_dto::{RoleDto, CreateRoleRequest, UpdateRoleRequest};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/{id}", get(get_role).put(update_role).delete(delete_role))
        .with_state(app)
}

async fn list_roles(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiR<Page<RoleDto>>>, AppError> {
    let repo = app.inject::<ToastyRoleRepository>();
    let (roles, total) = repo.find_page(1, query.keyword.as_deref(), query.page as u64, query.size as u64).await?;
    let dtos: Vec<RoleDto> = roles.iter().map(RoleDto::from).collect();
    Ok(Json(ApiR::success(Page::new(dtos, query.page, query.size, total as i64))))
}

async fn get_role(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiR<RoleDto>>, AppError> {
    let repo = app.inject::<ToastyRoleRepository>();
    let role = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::RoleNotFound, id.to_string()))?;
    Ok(Json(ApiR::success(RoleDto::from(&role))))
}

async fn create_role(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<ApiR<RoleDto>>, AppError> {
    let repo = app.inject::<ToastyRoleRepository>();
    if repo.find_by_code(&req.code).await?.is_some() { return Err(AppError::with_context(AdminErr::RoleCodeDuplicate, req.code)); }
    let mut role = crate::domain::role::Role::new(1, req.name, req.code, req.sort.unwrap_or(0));
    role.remark = req.remark;
    repo.save(&role).await?;
    Ok(Json(ApiR::success(RoleDto::from(&role))))
}

async fn update_role(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<ApiR<RoleDto>>, AppError> {
    let repo = app.inject::<ToastyRoleRepository>();
    let mut role = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::RoleNotFound, id.to_string()))?;
    if let Some(n) = req.name { role.name = n; }
    if let Some(r) = req.remark { role.remark = Some(r); }
    if let Some(s) = req.sort { role.sort = s; }
    repo.save(&role).await?;
    Ok(Json(ApiR::success(RoleDto::from(&role))))
}

async fn delete_role(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiRes>, AppError> {
    let repo = app.inject::<ToastyRoleRepository>();
    let role = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::RoleNotFound, id.to_string()))?;
    if role.is_built_in() { return Err(AdminErr::RoleBuiltIn.into()); }
    repo.delete(id).await?;
    Ok(Json(ApiRes::ok()))
}
