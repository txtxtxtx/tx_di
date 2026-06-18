//! 权限管理 HTTP API

use axum::Json;
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::permission::app_service::PermissionAppService;
use admin_proto::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse, UserPermissionItem,
    CreatePermissionRequest, UpdatePermissionRequest,
    ListPermissionsResponse, PermissionDetail, Empty,
};
use tx_common::{ApiR, ApiRes};
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;

pub fn router() -> Router {
    Router::new()
        // 原有查询接口
        .route("/check", post(check_permission))
        .route("/user-permissions", post(get_user_permissions))
        .route("/all", get(get_all_permissions))
        // 新增 CRUD 接口
        .route("/", post(create_permission))
        .route("/{id}", get(get_permission))
        .route("/{id}", put(update_permission))
        .route("/{id}", delete(delete_permission))
        .route("/list", get(list_permissions))
}

// ============================================================
// 原有查询 handlers
// ============================================================

async fn check_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<PermissionCheckRequest>,
) -> Result<ApiR<PermissionCheckResponse>, ApiErr> {
    ensure_permission("permission:view").await?;
    let r = perm.check_permission(req).await?;
    Ok(ApiR::success(r))
}

async fn get_user_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<GetUserPermissionsRequest>,
) -> Result<ApiR<UserPermissionsResponse>, ApiErr> {
    ensure_permission("permission:view").await?;
    let r = perm.get_user_permissions(req.user_id).await?;
    Ok(ApiR::success(r))
}

/// GET /api/permission/all
async fn get_all_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
) -> Result<ApiR<Vec<UserPermissionItem>>, ApiErr> {
    ensure_permission("permission:view").await?;
    let list = perm.get_all_permissions().await?;
    Ok(ApiR::success(list))
}

// ============================================================
// 新增 CRUD handlers
// ============================================================

/// POST /api/permission/
async fn create_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<CreatePermissionRequest>,
) -> Result<ApiR<PermissionDetail>, ApiErr> {
    ensure_permission("permission:create").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = perm.create_permission(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// GET /api/permission/{id}
async fn get_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<PermissionDetail>, ApiErr> {
    ensure_permission("permission:view").await?;
    let r = perm.get_permission(id).await?;
    Ok(ApiR::success(r))
}

/// PUT /api/permission/{id}
async fn update_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdatePermissionRequest>,
) -> Result<ApiR<PermissionDetail>, ApiErr> {
    ensure_permission("permission:update").await?;
    let mut req = req;
    req.id = id;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = perm.update_permission(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

/// DELETE /api/permission/{id}
async fn delete_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("permission:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    perm.delete_permission(id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

/// GET /api/permission/list
async fn list_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
) -> Result<ApiR<ListPermissionsResponse>, ApiErr> {
    ensure_permission("permission:view").await?;
    let list = perm.get_permission_list().await?;
    Ok(ApiR::success(ListPermissionsResponse {
        permissions: list,
    }))
}
