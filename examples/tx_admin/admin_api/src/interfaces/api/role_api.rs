//! 角色管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateRoleRequest, UpdateRoleRequest, DeleteRoleRequest, GetRoleRequest,
    ListRolesRequest, AssignMenusRequest, RoleResponse, Empty,
};
use crate::interfaces::dto::{ApiResponse, PageResponse};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_role))
        .route("/{role_id}", get(get_role))
        .route("/{role_id}", put(update_role))
        .route("/{role_id}", delete(delete_role))
        .route("/list", post(list_roles))
        .route("/assign-menus", post(assign_menus))
        .with_state(app)
}

/// POST /api/role/
async fn create_role(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<ApiResponse<RoleResponse>>, tx_error::AppError> {
    // TODO: 调用 RoleAppService::create
    let resp = RoleResponse {
        id: 1,
        name: req.name.clone(),
        code: req.code.clone(),
        sort: req.sort,
        data_scope: 0,
        status: 1,
        remark: req.remark.clone(),
        menu_ids: req.menu_ids.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/role/{role_id}
async fn get_role(
    State(_app): State<Arc<App>>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<RoleResponse>>, tx_error::AppError> {
    // TODO: 调用 RoleAppService::get_by_id
    let resp = RoleResponse {
        id: role_id,
        name: "placeholder".into(),
        code: "placeholder".into(),
        sort: 0,
        data_scope: 0,
        status: 1,
        remark: None,
        menu_ids: vec![],
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/role/{role_id}
async fn update_role(
    State(_app): State<Arc<App>>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateRoleRequest>,
) -> Result<Json<ApiResponse<RoleResponse>>, tx_error::AppError> {
    // TODO: 调用 RoleAppService::update
    req.role_id = role_id;
    let resp = RoleResponse {
        id: role_id,
        name: req.name.clone(),
        code: req.code.clone(),
        sort: req.sort,
        data_scope: req.data_scope,
        status: 1,
        remark: req.remark.clone(),
        menu_ids: vec![],
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/role/{role_id}
async fn delete_role(
    State(_app): State<Arc<App>>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 RoleAppService::delete
    let _ = role_id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/role/list
async fn list_roles(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListRolesRequest>,
) -> Result<Json<ApiResponse<PageResponse<RoleResponse>>>, tx_error::AppError> {
    // TODO: 调用 RoleAppService::list
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}

/// POST /api/role/assign-menus
async fn assign_menus(
    State(_app): State<Arc<App>>,
    Json(_req): Json<AssignMenusRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 RoleAppService::assign_menus
    Ok(Json(ApiResponse::success(Empty {})))
}
