//! 菜单管理 HTTP API

use axum::{Json, Router, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use tx_di_axum::bound::DiComp;
use admin_app::menu::app_service::MenuAppService;
use admin_proto::{CreateMenuRequest, UpdateMenuRequest, ListMenusRequest, MenuResponse};
use tx_common::{ApiR, ApiRes};

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_menu))
        .route("/{menu_id}", get(get_menu))
        .route("/{menu_id}", put(update_menu))
        .route("/{menu_id}", delete(delete_menu))
        .route("/list", post(list_menus))
}

fn map_menu(m: admin_app::menu::dto::MenuResponse) -> MenuResponse { MenuResponse { id: m.id, name: m.name, permission: m.permission, types: m.types, sort: m.sort, parent_id: m.parent_id, path: m.path, icon: m.icon, component: m.component, component_name: m.component_name, status: m.status, visible: m.visible, keep_alive: m.keep_alive } }

async fn create_menu(DiComp(menu): DiComp<MenuAppService>, Json(req): Json<CreateMenuRequest>) -> impl IntoResponse {
    let cmd = admin_app::menu::dto::CreateMenuCommand { name: req.name, permission: req.permission, types: req.types, sort: req.sort, parent_id: req.parent_id, path: None, icon: None, component: None, component_name: None };
    match menu.create_menu(cmd, None).await { Ok(r) => ApiR::success(map_menu(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn get_menu(DiComp(menu): DiComp<MenuAppService>, axum::extract::Path(menu_id): axum::extract::Path<u64>) -> impl IntoResponse {
    let query = admin_app::menu::dto::MenuQueryRequest { name: None, status: None, types: None };
    let list = match menu.get_menu_list(query).await {
        Ok(l) => l,
        Err(e) => return ApiRes::from(e).into_typed(),
    };
    let Some(r) = list.into_iter().find(|m| m.id == menu_id) else {
        return ApiRes::fail("not found".into()).into_typed();
    };
    ApiR::success(map_menu(r))
}

async fn update_menu(DiComp(menu): DiComp<MenuAppService>, axum::extract::Path(menu_id): axum::extract::Path<u64>, Json(req): Json<UpdateMenuRequest>) -> impl IntoResponse {
    let cmd = admin_app::menu::dto::UpdateMenuCommand { menu_id, name: req.name, permission: req.permission, types: req.types, sort: req.sort, parent_id: req.parent_id, path: req.path, icon: req.icon, component: req.component, component_name: req.component_name, visible: req.visible, keep_alive: req.keep_alive };
    match menu.update_menu(cmd, None).await { Ok(r) => ApiR::success(map_menu(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn delete_menu(DiComp(menu): DiComp<MenuAppService>, axum::extract::Path(menu_id): axum::extract::Path<u64>) -> impl IntoResponse {
    match menu.delete_menu(menu_id, None).await { Ok(()) => ApiRes::ok(), Err(e) => ApiRes::from(e) }
}

async fn list_menus(DiComp(menu): DiComp<MenuAppService>, Json(req): Json<ListMenusRequest>) -> impl IntoResponse {
    let query = admin_app::menu::dto::MenuQueryRequest { name: req.name, status: req.status, types: None };
    match menu.get_menu_tree(query).await { Ok(tree) => ApiR::success(tree), Err(e) => ApiRes::from(e).into_typed() }
}
