//! 权限管理 HTTP API

use axum::{Json, Router, extract::State, routing::post};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse,
};
use crate::interfaces::dto::ApiResponse;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/check", post(check_permission))
        .route("/user-permissions", post(get_user_permissions))
        .with_state(app)
}

/// POST /api/permission/check
async fn check_permission(
    State(_app): State<Arc<App>>,
    Json(_req): Json<PermissionCheckRequest>,
) -> Result<Json<ApiResponse<PermissionCheckResponse>>, tx_error::AppError> {
    // TODO: 调用 PermissionAppService::check
    let resp = PermissionCheckResponse { has_permission: true };
    Ok(Json(ApiResponse::success(resp)))
}

/// POST /api/permission/user-permissions
async fn get_user_permissions(
    State(_app): State<Arc<App>>,
    Json(_req): Json<GetUserPermissionsRequest>,
) -> Result<Json<ApiResponse<UserPermissionsResponse>>, tx_error::AppError> {
    // TODO: 调用 PermissionAppService::get_user_permissions
    let resp = UserPermissionsResponse {
        user_id: 0,
        permissions: vec![],
        items: vec![],
    };
    Ok(Json(ApiResponse::success(resp)))
}
