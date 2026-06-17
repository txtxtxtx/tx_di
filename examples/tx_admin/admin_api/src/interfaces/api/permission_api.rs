//! 权限管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
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
) -> Result<R<PermissionCheckResponse>, ApiErr> {
    ensure_permission("permission:view").await?;
    let cmd = admin_app::permission::dto::PermissionCheckRequest {
        user_id: req.user_id,
        permission: req.permission,
    };
    let r = perm.check_permission(cmd).await?;
    Ok(R(ApiR::success(PermissionCheckResponse { has_permission: r.has_permission })))
}

async fn get_user_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<GetUserPermissionsRequest>,
) -> Result<R<UserPermissionsResponse>, ApiErr> {
    ensure_permission("permission:view").await?;

    // 从数据库按用户 ID 查询权限编码，再按编码批量查询权限详情
    let (permissions, items) = perm.get_user_permission_items(req.user_id).await?;

    Ok(R(ApiR::success(UserPermissionsResponse {
        user_id: req.user_id,
        items: items.into_iter().map(|p| UserPermissionItem {
            code: p.code,
            name: p.name,
            permission_type: p.permission_type,
        }).collect(),
        permissions,
    })))
}

/// GET /api/permission/all
async fn get_all_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
) -> Result<R<Vec<UserPermissionItem>>, ApiErr> {
    ensure_permission("permission:view").await?;
    let list = perm.get_all_permissions().await?;
    Ok(R(ApiR::success(list.into_iter().map(|p| UserPermissionItem {
        code: p.code,
        name: p.name,
        permission_type: p.permission_type,
    }).collect())))
}

// ============================================================
// 新增 CRUD handlers
// ============================================================

fn map_permission_detail(r: admin_app::permission::dto::PermissionResponse) -> PermissionDetail {
    PermissionDetail {
        id: r.id,
        name: r.name,
        permission_code: r.permission_code,
        r#type: r.permission_type,
        parent_id: r.parent_id,
        sort: r.sort,
        description: r.description.unwrap_or_default(),
        status: r.status,
    }
}

/// POST /api/permission/
async fn create_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<CreatePermissionRequest>,
) -> Result<R<PermissionDetail>, ApiErr> {
    ensure_permission("permission:create").await?;
    let cmd = admin_app::permission::dto::CreatePermissionCommand {
        name: req.name,
        permission_code: req.permission_code,
        permission_type: req.r#type,
        parent_id: req.parent_id,
        sort: req.sort,
        description: if req.description.is_empty() { None } else { Some(req.description) },
    };
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = perm.create_permission(cmd, Some(login_id)).await?;
    Ok(R(ApiR::success(map_permission_detail(r))))
}

/// GET /api/permission/{id}
async fn get_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<R<PermissionDetail>, ApiErr> {
    ensure_permission("permission:view").await?;
    let r = perm.get_permission(id).await?;
    Ok(R(ApiR::success(map_permission_detail(r))))
}

/// PUT /api/permission/{id}
async fn update_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdatePermissionRequest>,
) -> Result<R<PermissionDetail>, ApiErr> {
    ensure_permission("permission:update").await?;
    let cmd = admin_app::permission::dto::UpdatePermissionCommand {
        id,
        name: req.name,
        permission_code: req.permission_code,
        permission_type: req.r#type,
        parent_id: req.parent_id,
        sort: req.sort,
        description: if req.description.is_empty() { None } else { Some(req.description) },
    };
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = perm.update_permission(cmd, Some(login_id)).await?;
    Ok(R(ApiR::success(map_permission_detail(r))))
}

/// DELETE /api/permission/{id}
async fn delete_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("permission:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    perm.delete_permission(id, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// GET /api/permission/list
async fn list_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
) -> Result<R<ListPermissionsResponse>, ApiErr> {
    ensure_permission("permission:view").await?;
    let list = perm.get_permission_list().await?;
    Ok(R(ApiR::success(ListPermissionsResponse {
        permissions: list.into_iter().map(map_permission_detail).collect(),
    })))
}
