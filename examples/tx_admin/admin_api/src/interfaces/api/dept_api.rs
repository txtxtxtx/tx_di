//! 部门管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateDeptRequest, UpdateDeptRequest, DeleteDeptRequest, GetDeptRequest,
    ListDeptsRequest, DeptResponse, Empty,
};
use crate::interfaces::dto::ApiResponse;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_dept))
        .route("/{dept_id}", get(get_dept))
        .route("/{dept_id}", put(update_dept))
        .route("/{dept_id}", delete(delete_dept))
        .route("/list", post(list_depts))
        .with_state(app)
}

/// POST /api/dept/
async fn create_dept(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateDeptRequest>,
) -> Result<Json<ApiResponse<DeptResponse>>, tx_error::AppError> {
    // TODO: 调用 DeptAppService::create
    let resp = DeptResponse {
        id: 1,
        name: req.name.clone(),
        parent_id: req.parent_id,
        sort: req.sort,
        leader_user_id: req.leader_user_id,
        phone: req.phone.clone(),
        email: req.email.clone(),
        status: 1,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/dept/{dept_id}
async fn get_dept(
    State(_app): State<Arc<App>>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<DeptResponse>>, tx_error::AppError> {
    // TODO: 调用 DeptAppService::get_by_id
    let resp = DeptResponse {
        id: dept_id,
        name: "placeholder".into(),
        parent_id: 0,
        sort: 0,
        leader_user_id: None,
        phone: None,
        email: None,
        status: 1,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/dept/{dept_id}
async fn update_dept(
    State(_app): State<Arc<App>>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateDeptRequest>,
) -> Result<Json<ApiResponse<DeptResponse>>, tx_error::AppError> {
    // TODO: 调用 DeptAppService::update
    req.dept_id = dept_id;
    let resp = DeptResponse {
        id: dept_id,
        name: req.name.clone(),
        parent_id: req.parent_id,
        sort: req.sort,
        leader_user_id: req.leader_user_id,
        phone: req.phone.clone(),
        email: req.email.clone(),
        status: 1,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/dept/{dept_id}
async fn delete_dept(
    State(_app): State<Arc<App>>,
    axum::extract::Path(dept_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 DeptAppService::delete
    let _ = dept_id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/dept/list
async fn list_depts(
    State(_app): State<Arc<App>>,
    Json(_req): Json<ListDeptsRequest>,
) -> Result<Json<ApiResponse<Vec<DeptResponse>>>, tx_error::AppError> {
    // TODO: 调用 DeptAppService::list (部门为树形，不分页)
    Ok(Json(ApiResponse::success(vec![])))
}
