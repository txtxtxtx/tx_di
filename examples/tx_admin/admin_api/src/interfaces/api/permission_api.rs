//! 权限管理 HTTP API

use axum::{Json, Router, extract::State, routing::post};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{PermissionCheckRequest, PermissionCheckResponse, GetUserPermissionsRequest};
use crate::services;
use tx_common::{ApiR, ApiRes};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/check", post(check_permission))
        .route("/user-permissions", post(get_user_permissions))
        .with_state(app)
}

async fn check_permission(
    Json(req): Json<PermissionCheckRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::permission::dto::PermissionCheckRequest {
        user_id: req.user_id,
        permission: req.permission,
    };
    match services::get().perm.check_permission(cmd).await {
        Ok(r) => ApiR::success(PermissionCheckResponse { has_permission: r.has_permission }),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_user_permissions(
    Json(req): Json<GetUserPermissionsRequest>,
) -> impl IntoResponse {
    match services::get().perm.get_user_permissions(req.user_id).await {
        Ok(r) => ApiR::success(admin_proto::UserPermissionsResponse {
            user_id: r.user_id,
            permissions: r.permissions,
            items: vec![],
        }),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
