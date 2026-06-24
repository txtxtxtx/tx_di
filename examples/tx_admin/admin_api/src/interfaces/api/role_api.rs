//! 角色管理 HTTP API

use axum::Json;
use tx_di_sa_token::StpUtil;
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::role::app_service::RoleAppService;
use admin_proto::{CreateRoleRequest, UpdateRoleRequest, AssignMenusRequest, ListRolesRequest, RoleResponse, Empty};
use admin_proto::UserResponse;
use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_role))
        .route("/{role_id}", get(get_role))
        .route("/{role_id}", put(update_role))
        .route("/{role_id}", delete(delete_role))
        .route("/list", post(list_roles))
        .route("/assign_menus", post(assign_menus))
        .route("/all", get(get_all_roles))
        .route("/{role_id}/users", get(get_role_users))
        .route("/{role_id}/users", post(add_users_to_role))
        .route("/{role_id}/users", delete(remove_users_from_role))
}

async fn create_role(
    DiComp(role): DiComp<RoleAppService>,
    Json(mut req): Json<CreateRoleRequest>,
) -> Result<ApiR<RoleResponse>, ApiErr> {
    ensure_permission("role:create").await?;
    use admin_app::empty_string::opt_filter;
    req.remark = opt_filter(req.remark);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = role.create_role(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn get_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<ApiR<RoleResponse>, ApiErr> {
    ensure_permission("role:view").await?;
    let r = role.get_role(role_id).await?;
    Ok(ApiR::success(r))
}

async fn update_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateRoleRequest>,
) -> Result<ApiR<RoleResponse>, ApiErr> {
    ensure_permission("role:update").await?;
    use admin_app::empty_string::opt_filter;
    req.role_id = role_id;
    req.remark = opt_filter(req.remark);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = role.update_role(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn delete_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("role:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    role.delete_role(role_id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_roles(
    DiComp(role): DiComp<RoleAppService>,
    Json(req): Json<ListRolesRequest>,
) -> Result<ApiR<Page<RoleResponse>>, ApiErr> {
    ensure_permission("role:view").await?;
    let page = role.get_role_page(req).await?;
    Ok(ApiR::success(page))
}

async fn assign_menus(
    DiComp(role): DiComp<RoleAppService>,
    Json(req): Json<AssignMenusRequest>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("role:assign_menu").await?;
    role.assign_menus(req.role_id, req.menu_ids).await?;
    Ok(ApiRes::ok().into_typed())
}

/// GET /api/role/all
async fn get_all_roles(
    DiComp(role): DiComp<RoleAppService>,
) -> Result<ApiR<Vec<RoleResponse>>, ApiErr> {
    ensure_permission("role:view").await?;
    let list = role.get_all_roles().await?;
    Ok(ApiR::success(list))
}

/// GET /api/role/{role_id}/users
async fn get_role_users(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
) -> Result<ApiR<Vec<UserResponse>>, ApiErr> {
    ensure_permission("role:view").await?;
    let users = role.get_role_users(role_id).await?;
    Ok(ApiR::success(users))
}

/// POST /api/role/{role_id}/users
async fn add_users_to_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(user_ids): Json<Vec<u64>>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("role:assign_menu").await?;
    role.add_users_to_role(role_id, user_ids).await?;
    Ok(ApiRes::ok().into_typed())
}

/// DELETE /api/role/{role_id}/users
async fn remove_users_from_role(
    DiComp(role): DiComp<RoleAppService>,
    axum::extract::Path(role_id): axum::extract::Path<u64>,
    Json(user_ids): Json<Vec<u64>>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("role:assign_menu").await?;
    role.remove_users_from_role(role_id, user_ids).await?;
    Ok(ApiRes::ok().into_typed())
}
