//! 用户管理 HTTP API
//!
//! 使用 admin_proto 生成的用户 DTO，
//! HTTP 协议层仅负责 JSON ↔ Proto DTO 转换，业务逻辑委托给 admin_app。

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateUserRequest, UpdateUserRequest, DeleteUserRequest, GetUserRequest,
    ListUsersRequest, ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest,
    UserResponse, ListUsersResponse, Empty,
};
use crate::interfaces::dto::{ApiResponse, PageResponse};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_user))
        .route("/{user_id}", get(get_user))
        .route("/{user_id}", put(update_user))
        .route("/{user_id}", delete(delete_user))
        .route("/list", post(list_users))
        .route("/change-password", post(change_password))
        .route("/assign-roles", post(assign_roles))
        .route("/assign-depts", post(assign_depts))
        .with_state(app)
}

/// POST /api/user/
async fn create_user(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::create
    let resp = UserResponse {
        id: 1,
        username: req.username.clone(),
        nickname: req.nickname.clone(),
        email: req.email.clone(),
        mobile: req.mobile.clone(),
        sex: req.sex.unwrap_or(0),
        status: 1,
        remark: req.remark.clone(),
        role_ids: req.role_ids.clone(),
        dept_ids: req.dept_ids.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/user/{user_id}
async fn get_user(
    State(_app): State<Arc<App>>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<UserResponse>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::get_by_id
    let resp = UserResponse {
        id: user_id,
        username: "placeholder".into(),
        nickname: "Placeholder".into(),
        email: None,
        mobile: None,
        sex: 0,
        status: 1,
        remark: None,
        role_ids: vec![],
        dept_ids: vec![],
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/user/{user_id}
async fn update_user(
    State(_app): State<Arc<App>>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::update
    req.user_id = user_id;
    let resp = UserResponse {
        id: user_id,
        username: String::new(),
        nickname: req.nickname.clone(),
        email: req.email.clone(),
        mobile: req.mobile.clone(),
        sex: req.sex,
        status: 1,
        remark: req.remark.clone(),
        role_ids: vec![],
        dept_ids: vec![],
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/user/{user_id}
async fn delete_user(
    State(_app): State<Arc<App>>,
    axum::extract::Path(user_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::delete
    let _ = user_id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/user/list
async fn list_users(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListUsersRequest>,
) -> Result<Json<ApiResponse<PageResponse<UserResponse>>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::list
    let page = crate::interfaces::dto::PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}

/// POST /api/user/change-password
async fn change_password(
    State(_app): State<Arc<App>>,
    Json(_req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::change_password
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/user/assign-roles
async fn assign_roles(
    State(_app): State<Arc<App>>,
    Json(_req): Json<AssignRolesRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::assign_roles
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/user/assign-depts
async fn assign_depts(
    State(_app): State<Arc<App>>,
    Json(_req): Json<AssignDeptsRequest>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 UserAppService::assign_depts
    Ok(Json(ApiResponse::success(Empty {})))
}
