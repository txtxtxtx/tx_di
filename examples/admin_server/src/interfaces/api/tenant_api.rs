//! 租户管理 API

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tx_di_core::App;

use crate::application::tenant::{CreateTenantRequest, TenantService, UpdateTenantRequest};
use crate::infrastructure::persistence::InMemoryTenantRepository;
use crate::interfaces::dto::common::{ApiResponse, PageQuery, PageResponse};

/// 租户路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_tenants).post(create_tenant))
        .route("/packages", get(list_packages))
        .route("/{id}", get(get_tenant).put(update_tenant).delete(delete_tenant))
        .with_state(app)
}

async fn list_tenants(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Json<ApiResponse<PageResponse<crate::domain::tenant::Tenant>>> {
    let service = app.inject::<TenantService>();

    match service
        .list_tenants(query.keyword.as_deref(), None, query.page, query.page_size)
        .await
    {
        Ok((tenants, total)) => {
            Json(ApiResponse::success(PageResponse::new(tenants, total, query.page, query.page_size)))
        }
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

async fn get_tenant(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<crate::domain::tenant::Tenant>> {
    let repo = app.inject::<InMemoryTenantRepository>();
    match repo.find_by_id(&id).await {
        Ok(Some(tenant)) => Json(ApiResponse::success(tenant)),
        Ok(None) => Json(ApiResponse::error(404, "租户不存在")),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

async fn create_tenant(
    State(app): State<Arc<App>>,
    Json(body): Json<CreateTenantRequest>,
) -> Json<ApiResponse<crate::domain::tenant::Tenant>> {
    let service = app.inject::<TenantService>();

    match service.create_tenant(body).await {
        Ok(tenant) => Json(ApiResponse::success(tenant)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

async fn update_tenant(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTenantRequest>,
) -> Json<ApiResponse<crate::domain::tenant::Tenant>> {
    let service = app.inject::<TenantService>();

    match service.update_tenant(&id, body).await {
        Ok(tenant) => Json(ApiResponse::success(tenant)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

async fn delete_tenant(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let service = app.inject::<TenantService>();

    match service.delete_tenant(&id).await {
        Ok(()) => Json(ApiResponse::ok()),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

async fn list_packages(
    State(app): State<Arc<App>>,
) -> Json<ApiResponse<Vec<crate::domain::tenant::TenantPackage>>> {
    let service = app.inject::<TenantService>();

    match service.list_packages().await {
        Ok(packages) => Json(ApiResponse::success(packages)),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}
