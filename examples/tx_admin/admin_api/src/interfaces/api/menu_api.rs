//! 菜单管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::menu::app_service::MenuAppService;
use admin_proto::{CreateMenuRequest, UpdateMenuRequest, ListMenusRequest, MenuResponse, Empty};
use admin_domain::menu::model::value_object::MenuTreeNode;
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        .api_route("/", post(create_menu))
        .api_route("/{menu_id}", get(get_menu))
        .api_route("/{menu_id}", put(update_menu))
        .api_route("/{menu_id}", delete(delete_menu))
        .api_route("/list", post(list_menus))
}

fn map_menu(m: admin_app::menu::dto::MenuResponse) -> MenuResponse { MenuResponse { id: m.id, name: m.name, permission: m.permission, types: m.types, sort: m.sort, parent_id: m.parent_id, path: m.path, icon: m.icon, component: m.component, component_name: m.component_name, status: m.status, visible: m.visible, keep_alive: m.keep_alive } }

async fn create_menu(DiComp(menu): DiComp<MenuAppService>, Json(req): Json<CreateMenuRequest>) -> R<MenuResponse> {
    let cmd = admin_app::menu::dto::CreateMenuCommand { name: req.name, permission: req.permission, types: req.types, sort: req.sort, parent_id: req.parent_id, path: None, icon: None, component: None, component_name: None };
    match menu.create_menu(cmd, None).await { Ok(r) => R(ApiR::success(map_menu(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn get_menu(DiComp(menu): DiComp<MenuAppService>, axum::extract::Path(menu_id): axum::extract::Path<u64>) -> R<MenuResponse> {
    let query = admin_app::menu::dto::MenuQueryRequest { name: None, status: None, types: None };
    let list = match menu.get_menu_list(query).await {
        Ok(l) => l,
        Err(e) => return R(ApiRes::from(e).into_typed()),
    };
    let Some(r) = list.into_iter().find(|m| m.id == menu_id) else {
        return R(ApiRes::fail("not found".into()).into_typed());
    };
    R(ApiR::success(map_menu(r)))
}

async fn update_menu(DiComp(menu): DiComp<MenuAppService>, axum::extract::Path(menu_id): axum::extract::Path<u64>, Json(req): Json<UpdateMenuRequest>) -> R<MenuResponse> {
    let cmd = admin_app::menu::dto::UpdateMenuCommand { menu_id, name: req.name, permission: req.permission, types: req.types, sort: req.sort, parent_id: req.parent_id, path: req.path, icon: req.icon, component: req.component, component_name: req.component_name, visible: req.visible, keep_alive: req.keep_alive };
    match menu.update_menu(cmd, None).await { Ok(r) => R(ApiR::success(map_menu(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn delete_menu(DiComp(menu): DiComp<MenuAppService>, axum::extract::Path(menu_id): axum::extract::Path<u64>) -> R<Empty> {
    match menu.delete_menu(menu_id, None).await { Ok(()) => R(ApiRes::ok().into_typed()), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn list_menus(DiComp(menu): DiComp<MenuAppService>, Json(req): Json<ListMenusRequest>) -> R<Vec<MenuTreeNode>> {
    let query = admin_app::menu::dto::MenuQueryRequest { name: req.name, status: req.status, types: None };
    match menu.get_menu_tree(query).await { Ok(tree) => R(ApiR::success(tree)), Err(e) => R(ApiRes::from(e).into_typed()) }
}
