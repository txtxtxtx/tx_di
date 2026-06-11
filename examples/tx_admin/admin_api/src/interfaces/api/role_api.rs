//! 角色管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{CreateRoleRequest, UpdateRoleRequest, AssignMenusRequest, ListRolesRequest, RoleResponse, Empty};
use crate::services;
use tx_common::{ApiR, ApiRes, Page};

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

fn map_role(r: admin_app::role::dto::RoleResponse) -> RoleResponse {
    RoleResponse {
        id: r.id,
        name: r.name,
        code: r.code,
        sort: r.sort,
        data_scope: r.data_scope,
        status: r.status,
        remark: r.remark,
        menu_ids: r.menu_ids,
    }
}

/// POST /api/role/
async fn create_role(
    Json(req): Json<CreateRoleRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::role::dto::CreateRoleCommand {
        name: req.name,
        code: req.code,
        sort: req.sort,
        remark: req.remark,
        menu_ids: if req.menu_ids.is_empty() { None } else { Some(req.menu_ids) },
    };
    match services::get().role.create_role(cmd, None).await {
        Ok(r) => ApiR::success(map_role(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// GET /api/role/{role_id}
async fn get_role(
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().role.get_role(role_id).await {
        Ok(r) => ApiR::success(map_role(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// PUT /api/role/{role_id}
async fn update_role(
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::role::dto::UpdateRoleCommand {
        role_id,
        name: req.name,
        code: req.code,
        sort: req.sort,
        data_scope: req.data_scope,
        remark: req.remark,
    };
    match services::get().role.update_role(cmd, None).await {
        Ok(r) => ApiR::success(map_role(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// DELETE /api/role/{role_id}
async fn delete_role(
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().role.delete_role(role_id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

/// POST /api/role/list
async fn list_roles(
    Json(req): Json<ListRolesRequest>,
) -> impl IntoResponse {
    let query = admin_app::role::dto::RoleQueryRequest {
        name: req.name,
        code: req.code,
        status: req.status,
        page: req.page,
        size: req.page_size,
    };
    match services::get().role.get_role_page(query).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_role).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

/// POST /api/role/assign-menus
async fn assign_menus(
    Json(req): Json<AssignMenusRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::role::dto::AssignMenusCommand {
        role_id: req.role_id,
        menu_ids: req.menu_ids,
    };
    match services::get().role.assign_menus(cmd).await {
        Ok(_) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}
