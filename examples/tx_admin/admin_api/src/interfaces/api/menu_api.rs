//! 菜单管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{CreateMenuRequest, UpdateMenuRequest, ListMenusRequest, MenuResponse, Empty};
use crate::services;
use tx_common::{ApiR, ApiRes};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_menu))
        .route("/{menu_id}", get(get_menu))
        .route("/{menu_id}", put(update_menu))
        .route("/{menu_id}", delete(delete_menu))
        .route("/list", post(list_menus))
        .with_state(app)
}

fn map_menu(m: admin_app::menu::dto::MenuResponse) -> MenuResponse {
    MenuResponse {
        id: m.id, name: m.name, permission: m.permission,
        types: m.types, sort: m.sort, parent_id: m.parent_id,
        path: m.path, icon: m.icon, component: m.component,
        component_name: m.component_name, status: m.status,
        visible: m.visible, keep_alive: m.keep_alive,
    }
}

async fn create_menu(
    Json(req): Json<CreateMenuRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::menu::dto::CreateMenuCommand {
        name: req.name, permission: req.permission, types: req.types,
        sort: req.sort, parent_id: req.parent_id, path: req.path,
        icon: req.icon, component: req.component, component_name: req.component_name,
    };
    match services::get().menu.create_menu(cmd, None).await {
        Ok(r) => ApiR::success(map_menu(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_menu(
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let query = admin_app::menu::dto::MenuQueryRequest { name: None, status: None, types: None };
    match services::get().menu.get_menu_list(query).await {
        Ok(list) => {
            match list.into_iter().find(|m| m.id == menu_id) {
                Some(m) => ApiR::success(map_menu(m)),
                None => ApiRes::error(404, "菜单不存在".into()).into_typed(),
            }
        }
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn update_menu(
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateMenuRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::menu::dto::UpdateMenuCommand {
        menu_id, name: req.name, permission: req.permission, types: req.types,
        sort: req.sort, parent_id: req.parent_id, path: req.path,
        icon: req.icon, component: req.component, component_name: req.component_name,
        visible: req.visible, keep_alive: req.keep_alive,
    };
    match services::get().menu.update_menu(cmd, None).await {
        Ok(r) => ApiR::success(map_menu(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn delete_menu(
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().menu.delete_menu(menu_id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_menus(
    Json(req): Json<ListMenusRequest>,
) -> impl IntoResponse {
    let query = admin_app::menu::dto::MenuQueryRequest {
        name: req.name, status: req.status, types: req.types,
    };
    match services::get().menu.get_menu_list(query).await {
        Ok(list) => ApiR::success(list.into_iter().map(map_menu).collect::<Vec<_>>()),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
