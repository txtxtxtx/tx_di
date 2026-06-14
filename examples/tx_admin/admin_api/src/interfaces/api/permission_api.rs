//! 权限管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::permission::app_service::PermissionAppService;
use admin_proto::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse, UserPermissionItem,
    CreatePermissionRequest, UpdatePermissionRequest,
    ListPermissionsResponse, PermissionDetail, Empty,
};
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        // 原有查询接口
        .api_route("/check", post(check_permission))
        .api_route("/user-permissions", post(get_user_permissions))
        .api_route("/all", get(get_all_permissions))
        // 新增 CRUD 接口
        .api_route("/", post(create_permission))
        .api_route("/{id}", get(get_permission))
        .api_route("/{id}", put(update_permission))
        .api_route("/{id}", delete(delete_permission))
        .api_route("/list", get(list_permissions))
}

// ============================================================
// 原有查询 handlers
// ============================================================

async fn check_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<PermissionCheckRequest>,
) -> R<PermissionCheckResponse> {
    let cmd = admin_app::permission::dto::PermissionCheckRequest {
        user_id: req.user_id,
        permission: req.permission,
    };
    match perm.check_permission(cmd).await {
        Ok(r) => R(ApiR::success(PermissionCheckResponse { has_permission: r.has_permission })),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

async fn get_user_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
    Json(req): Json<GetUserPermissionsRequest>,
) -> R<UserPermissionsResponse> {
    match perm.get_user_permissions(req.user_id).await {
        Ok(r) => R(ApiR::success(UserPermissionsResponse {
            user_id: r.user_id,
            items: vec![],
            permissions: r.permissions,
        })),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// GET /api/permission/all
async fn get_all_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
) -> R<Vec<UserPermissionItem>> {
    match perm.get_all_permissions().await {
        Ok(list) => R(ApiR::success(list.into_iter().map(|p| UserPermissionItem {
            code: p.code,
            name: p.name,
            permission_type: p.permission_type,
        }).collect())),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
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
) -> R<PermissionDetail> {
    let cmd = admin_app::permission::dto::CreatePermissionCommand {
        name: req.name,
        permission_code: req.permission_code,
        permission_type: req.r#type,
        parent_id: req.parent_id,
        sort: req.sort,
        description: if req.description.is_empty() { None } else { Some(req.description) },
    };
    match perm.create_permission(cmd, None).await {
        Ok(r) => R(ApiR::success(map_permission_detail(r))),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// GET /api/permission/{id}
async fn get_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> R<PermissionDetail> {
    match perm.get_permission(id).await {
        Ok(r) => R(ApiR::success(map_permission_detail(r))),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// PUT /api/permission/{id}
async fn update_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(req): Json<UpdatePermissionRequest>,
) -> R<PermissionDetail> {
    let cmd = admin_app::permission::dto::UpdatePermissionCommand {
        id,
        name: req.name,
        permission_code: req.permission_code,
        permission_type: req.r#type,
        parent_id: req.parent_id,
        sort: req.sort,
        description: if req.description.is_empty() { None } else { Some(req.description) },
    };
    match perm.update_permission(cmd, None).await {
        Ok(r) => R(ApiR::success(map_permission_detail(r))),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// DELETE /api/permission/{id}
async fn delete_permission(
    DiComp(perm): DiComp<PermissionAppService>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> R<Empty> {
    match perm.delete_permission(id, None).await {
        Ok(()) => R(ApiRes::ok().into_typed()),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}

/// GET /api/permission/list
async fn list_permissions(
    DiComp(perm): DiComp<PermissionAppService>,
) -> R<ListPermissionsResponse> {
    match perm.get_permission_list().await {
        Ok(list) => R(ApiR::success(ListPermissionsResponse {
            permissions: list.into_iter().map(map_permission_detail).collect(),
        })),
        Err(e) => R(ApiRes::from(e).into_typed()),
    }
}
