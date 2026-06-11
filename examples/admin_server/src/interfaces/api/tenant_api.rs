//! 租户管理 API

use axum::{Json, Router, extract::{Path, Query, State}, routing::{delete, get, post, put}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::{AppError, AdminErr};
use crate::domain::tenant::TenantRepository;
use crate::domain::tenant::repo::ToastyTenantRepository;
use tx_common::{ApiR, ApiRes, Page};
use crate::interfaces::dto::common::PageQuery;
use crate::interfaces::dto::tenant_dto::{TenantDto, CreateTenantRequest, UpdateTenantRequest};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_tenants).post(create_tenant))
        .route("/{id}", get(get_tenant).put(update_tenant).delete(delete_tenant))
        .with_state(app)
}

async fn list_tenants(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiR<Page<TenantDto>>>, AppError> {
    let repo = app.inject::<ToastyTenantRepository>();
    let (tenants, total) = repo.find_page(query.keyword.as_deref(), None, query.page as u64, query.size as u64).await?;
    let dtos: Vec<TenantDto> = tenants.iter().map(TenantDto::from).collect();
    Ok(Json(ApiR::success(Page::new(dtos, query.page, query.size, total as i64))))
}

async fn get_tenant(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiR<TenantDto>>, AppError> {
    let repo = app.inject::<ToastyTenantRepository>();
    let tenant = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::TenantNotFound, id.to_string()))?;
    Ok(Json(ApiR::success(TenantDto::from(&tenant))))
}

async fn create_tenant(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<ApiR<TenantDto>>, AppError> {
    let repo = app.inject::<ToastyTenantRepository>();
    let mut tenant = crate::domain::tenant::Tenant::new(req.name);
    tenant.contact_name = req.contact_name; tenant.contact_mobile = req.contact_mobile; tenant.package_id = req.package_id;
    repo.save(&tenant).await?;
    Ok(Json(ApiR::success(TenantDto::from(&tenant))))
}

async fn update_tenant(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<ApiR<TenantDto>>, AppError> {
    let repo = app.inject::<ToastyTenantRepository>();
    let mut tenant = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::TenantNotFound, id.to_string()))?;
    if let Some(n) = req.name { tenant.name = n; }
    if let Some(cn) = req.contact_name { tenant.contact_name = Some(cn); }
    if let Some(cm) = req.contact_mobile { tenant.contact_mobile = Some(cm); }
    if let Some(p) = req.package_id { tenant.package_id = Some(p); }
    repo.save(&tenant).await?;
    Ok(Json(ApiR::success(TenantDto::from(&tenant))))
}

async fn delete_tenant(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiRes>, AppError> {
    let repo = app.inject::<ToastyTenantRepository>();
    repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::TenantNotFound, id.to_string()))?;
    repo.delete(id).await?;
    Ok(Json(ApiRes::ok()))
}
