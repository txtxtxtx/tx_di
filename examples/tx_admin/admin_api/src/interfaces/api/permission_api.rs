//! 权限管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::post;
use tx_di_axum::bound::DiComp;
use admin_app::permission::app_service::PermissionAppService;
use admin_proto::{PermissionCheckRequest, PermissionCheckResponse, GetUserPermissionsRequest, UserPermissionsResponse};
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        .api_route("/check", post(check_permission))
        .api_route("/user-permissions", post(get_user_permissions))
}

async fn check_permission(DiComp(perm): DiComp<PermissionAppService>, Json(req): Json<PermissionCheckRequest>) -> R<PermissionCheckResponse> {
    let cmd = admin_app::permission::dto::PermissionCheckRequest { user_id: req.user_id, permission: req.permission };
    match perm.check_permission(cmd).await {
        Ok(r) => R(ApiR::success(PermissionCheckResponse { has_permission: r.has_permission })),
        Err(e) => R(ApiRes::from(e).into_typed())
    }
}

async fn get_user_permissions(DiComp(perm): DiComp<PermissionAppService>, Json(req): Json<GetUserPermissionsRequest>) -> R<UserPermissionsResponse> {
    match perm.get_user_permissions(req.user_id).await {
        Ok(r) => R(ApiR::success(UserPermissionsResponse { user_id: r.user_id, items: vec![], permissions: r.permissions })),
        Err(e) => R(ApiRes::from(e).into_typed())
    }
}
