//! 角色管理 API

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tx_di_core::App;

use crate::application::role::{CreateRoleRequest, RoleService, UpdateRoleRequest};
use crate::infrastructure::persistence::InMemoryRoleRepository;
use crate::interfaces::dto::common::{ApiResponse, PageQuery, PageResponse};

/// 角色路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/all", get(all_roles))
        .route("/{id}", get(get_role).put(update_role).delete(delete_role))
        .with_state(app)
}

async fn list_roles(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Json<ApiResponse<PageResponse<crate::domain::role::Role>>> {
    let service = app.inject::<RoleService>();

    match service
        .list_roles("t-default-001", query.keyword.as_deref(), query.page, query.page_size)
        .await
    {
        Ok((roles, total)) => {
            Json(ApiResponse::success(PageResponse::new(roles, total, query.page, query.page_size)))
        }
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

async fn all_roles(
    State(app): State<Arc<App>>,
) -> Json<ApiResponse<Vec<crate::domain::role::Role>>> {
    let service = app.inject::<RoleService>();

    match service.all_roles("t-default-001").await {
        Ok(roles) => Json(ApiResponse::success(roles)),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

async fn get_role(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<crate::domain::role::Role>> {
    let repo = app.inject::<InMemoryRoleRepository>();
    match repo.find_by_id(&id).await {
        Ok(Some(role)) => Json(ApiResponse::success(role)),
        Ok(None) => Json(ApiResponse::error(404, "角色不存在")),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

async fn create_role(
    State(app): State<Arc<App>>,
    Json(body): Json<CreateRoleRequest>,
) -> Json<ApiResponse<crate::domain::role::Role>> {
    let service = app.inject::<RoleService>();

    match service.create_role("t-default-001", body).await {
        Ok(role) => Json(ApiResponse::success(role)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

async fn update_role(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRoleRequest>,
) -> Json<ApiResponse<crate::domain::role::Role>> {
    let service = app.inject::<RoleService>();

    match service.update_role(&id, body).await {
        Ok(role) => Json(ApiResponse::success(role)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

async fn delete_role(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let service = app.inject::<RoleService>();

    match service.delete_role(&id).await {
        Ok(()) => Json(ApiResponse::ok()),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}
