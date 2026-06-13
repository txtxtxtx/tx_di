//! 角色管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::role::app_service::RoleAppService;
use admin_proto::{CreateRoleRequest, UpdateRoleRequest, AssignMenusRequest, ListRolesRequest, RoleResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};

pub fn router() -> Router {
    Router::new()
        .api_route("/", post(create_role))
        .api_route("/{role_id}", get(get_role))
        .api_route("/{role_id}", put(update_role))
        .api_route("/{role_id}", delete(delete_role))
        .api_route("/list", post(list_roles))
        .api_route("/assign-menus", post(assign_menus))
}

fn map_role(r: admin_app::role::dto::RoleResponse) -> RoleResponse {
    RoleResponse { id: r.id, name: r.name, code: r.code, sort: r.sort, data_scope: r.data_scope, status: r.status, remark: r.remark, menu_ids: r.menu_ids }
}

async fn create_role(DiComp(role): DiComp<RoleAppService>, Json(req): Json<CreateRoleRequest>) -> R<RoleResponse> {
    let cmd = admin_app::role::dto::CreateRoleCommand { name: req.name, code: req.code, sort: req.sort, remark: req.remark, menu_ids: if req.menu_ids.is_empty() { None } else { Some(req.menu_ids) } };
    match role.create_role(cmd, None).await { Ok(r) => R(ApiR::success(map_role(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn get_role(DiComp(role): DiComp<RoleAppService>, axum::extract::Path(role_id): axum::extract::Path<u64>) -> R<RoleResponse> {
    match role.get_role(role_id).await { Ok(r) => R(ApiR::success(map_role(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn update_role(DiComp(role): DiComp<RoleAppService>, axum::extract::Path(role_id): axum::extract::Path<u64>, Json(req): Json<UpdateRoleRequest>) -> R<RoleResponse> {
    let cmd = admin_app::role::dto::UpdateRoleCommand { role_id, name: req.name, code: req.code, sort: req.sort, data_scope: req.data_scope, remark: req.remark };
    match role.update_role(cmd, None).await { Ok(r) => R(ApiR::success(map_role(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn delete_role(DiComp(role): DiComp<RoleAppService>, axum::extract::Path(role_id): axum::extract::Path<u64>) -> R<Empty> {
    match role.delete_role(role_id, None).await { Ok(()) => R(ApiRes::ok().into_typed()), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn list_roles(DiComp(role): DiComp<RoleAppService>, Json(req): Json<ListRolesRequest>) -> R<Page<RoleResponse>> {
    let query = admin_app::role::dto::RoleQueryRequest { name: req.name, code: req.code, status: req.status, page: req.page, size: req.page_size };
    match role.get_role_page(query).await { Ok(page) => R(ApiR::success(Page::new(page.list.into_iter().map(map_role).collect(), page.page, page.size, page.total))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn assign_menus(DiComp(role): DiComp<RoleAppService>, Json(req): Json<AssignMenusRequest>) -> R<Empty> {
    let cmd = admin_app::role::dto::AssignMenusCommand { role_id: req.role_id, menu_ids: req.menu_ids };
    match role.assign_menus(cmd).await { Ok(_) => R(ApiRes::ok().into_typed()), Err(e) => R(ApiRes::from(e).into_typed()) }
}
