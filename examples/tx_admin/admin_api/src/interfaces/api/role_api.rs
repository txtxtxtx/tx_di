//! 角色管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::role::app_service::RoleAppService;
use admin_proto::{CreateRoleRequest, UpdateRoleRequest, AssignMenusRequest, ListRolesRequest, RoleResponse, Empty};
use admin_proto::UserResponse;
use tx_common::{ApiR, ApiRes, Page};
use tx_di_sa_token::sa_check_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_role))
        .route("/{role_id}", get(get_role))
        .route("/{role_id}", put(update_role))
        .route("/{role_id}", delete(delete_role))
        .route("/list", post(list_roles))
        .route("/assign-menus", post(assign_menus))
        .route("/all", get(get_all_roles))
        .route("/{role_id}/users", get(get_role_users))
        .route("/{role_id}/users", post(add_users_to_role))
        .route("/{role_id}/users", delete(remove_users_from_role))
}

fn map_role(r: admin_app::role::dto::RoleResponse) -> RoleResponse {
    RoleResponse { id: r.id, name: r.name, code: r.code, sort: r.sort, data_scope: r.data_scope, status: r.status, remark: r.remark, menu_ids: r.menu_ids }
}

#[sa_check_permission("role:create")]
async fn create_role(
    DiComp(role): DiComp<RoleAppService>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<R<RoleResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::role::dto::CreateRoleCommand { name: req.name, code: req.code, sort: req.sort, remark: opt_filter(req.remark), menu_ids: if req.menu_ids.is_empty() { None } else { Some(req.menu_ids) } };
    let r = role.create_role(cmd, None).await?;
    Ok(R(ApiR::success(map_role(r))))
}

#[sa_check_permission("role:view")]
async fn get_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<R<RoleResponse>, ApiErr> {
    let r = role.get_role(role_id).await?;
    Ok(R(ApiR::success(map_role(r))))
}

#[sa_check_permission("role:update")]
async fn update_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<R<RoleResponse>, ApiErr> {
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::role::dto::UpdateRoleCommand { role_id, name: req.name, code: req.code, sort: req.sort, data_scope: req.data_scope, remark: opt_filter(req.remark) };
    let r = role.update_role(cmd, None).await?;
    Ok(R(ApiR::success(map_role(r))))
}

#[sa_check_permission("role:delete")]
async fn delete_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    role.delete_role(role_id, None).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

#[sa_check_permission("role:view")]
async fn list_roles(
    DiComp(role): DiComp<RoleAppService>,
    Json(req): Json<ListRolesRequest>,
) -> Result<R<Page<RoleResponse>>, ApiErr> {
    let query = admin_app::role::dto::RoleQueryRequest { name: req.name, code: req.code, status: req.status, page: req.page, size: req.page_size };
    let page = role.get_role_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_role).collect(), page.page, page.size, page.total))))
}

#[sa_check_permission("role:assign_menu")]
async fn assign_menus(
    DiComp(role): DiComp<RoleAppService>,
    Json(req): Json<AssignMenusRequest>,
) -> Result<R<Empty>, ApiErr> {
    let cmd = admin_app::role::dto::AssignMenusCommand { role_id: req.role_id, menu_ids: req.menu_ids };
    role.assign_menus(cmd).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// GET /api/role/all
#[sa_check_permission("role:view")]
async fn get_all_roles(
    DiComp(role): DiComp<RoleAppService>,
) -> Result<R<Vec<RoleResponse>>, ApiErr> {
    let list = role.get_all_roles().await?;
    Ok(R(ApiR::success(list.into_iter().map(map_role).collect())))
}

/// GET /api/role/{role_id}/users
#[sa_check_permission("role:view")]
async fn get_role_users(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<R<Vec<UserResponse>>, ApiErr> {
    let users = role.get_role_users(role_id).await?;
    Ok(R(ApiR::success(users)))
}

/// POST /api/role/{role_id}/users
#[sa_check_permission("role:assign_menu")]
async fn add_users_to_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(user_ids): Json<Vec<u64>>,
) -> Result<R<Empty>, ApiErr> {
    role.add_users_to_role(role_id, user_ids).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

/// DELETE /api/role/{role_id}/users
#[sa_check_permission("role:assign_menu")]
async fn remove_users_from_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(user_ids): Json<Vec<u64>>,
) -> Result<R<Empty>, ApiErr> {
    role.remove_users_from_role(role_id, user_ids).await?;
    Ok(R(ApiRes::ok().into_typed()))
}
