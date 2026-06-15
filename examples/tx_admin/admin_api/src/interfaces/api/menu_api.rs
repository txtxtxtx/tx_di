//! 菜单管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::menu::app_service::MenuAppService;
use admin_proto::{CreateMenuRequest, UpdateMenuRequest, ListMenusRequest, MenuResponse, Empty};
use admin_domain::menu::model::value_object::MenuTreeNode;
use tx_common::{ApiR, ApiRes};
use crate::auth::ensure_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_menu))
        .route("/{menu_id}", get(get_menu))
        .route("/{menu_id}", put(update_menu))
        .route("/{menu_id}", delete(delete_menu))
        .route("/list", post(list_menus))
}

fn map_menu(m: admin_app::menu::dto::MenuResponse) -> MenuResponse { MenuResponse { id: m.id, name: m.name, permission: m.permission, types: m.types, sort: m.sort, parent_id: m.parent_id, path: m.path, icon: m.icon, component: m.component, component_name: m.component_name, status: m.status, visible: m.visible, keep_alive: m.keep_alive } }

async fn create_menu(
    DiComp(menu): DiComp<MenuAppService>,
    Json(req): Json<CreateMenuRequest>,
) -> Result<R<MenuResponse>, ApiErr> {
    ensure_permission("menu:create").await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::menu::dto::CreateMenuCommand { name: req.name, permission: req.permission, types: req.types, sort: req.sort, parent_id: req.parent_id, path: opt_filter(req.path), icon: opt_filter(req.icon), component: opt_filter(req.component), component_name: opt_filter(req.component_name) };
    let r = menu.create_menu(cmd, None).await?;
    Ok(R(ApiR::success(map_menu(r))))
}

async fn get_menu(
    DiComp(menu): DiComp<MenuAppService>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> Result<R<MenuResponse>, ApiErr> {
    ensure_permission("menu:view").await?;
    let query = admin_app::menu::dto::MenuQueryRequest { name: None, status: None, types: None };
    let list = menu.get_menu_list(query).await?;
    let r = list.into_iter().find(|m| m.id == menu_id)
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(R(ApiR::success(map_menu(r))))
}

async fn update_menu(
    DiComp(menu): DiComp<MenuAppService>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateMenuRequest>,
) -> Result<R<MenuResponse>, ApiErr> {
    ensure_permission("menu:update").await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::menu::dto::UpdateMenuCommand { menu_id, name: req.name, permission: req.permission, types: req.types, sort: req.sort, parent_id: req.parent_id, path: opt_filter(req.path), icon: opt_filter(req.icon), component: opt_filter(req.component), component_name: opt_filter(req.component_name), visible: req.visible, keep_alive: req.keep_alive };
    let r = menu.update_menu(cmd, None).await?;
    Ok(R(ApiR::success(map_menu(r))))
}

async fn delete_menu(
    DiComp(menu): DiComp<MenuAppService>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("menu:delete").await?;
    menu.delete_menu(menu_id, None).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

async fn list_menus(
    DiComp(menu): DiComp<MenuAppService>,
    Json(req): Json<ListMenusRequest>,
) -> Result<R<Vec<MenuTreeNode>>, ApiErr> {
    ensure_permission("menu:view").await?;
    let query = admin_app::menu::dto::MenuQueryRequest { name: req.name, status: req.status, types: None };
    let tree = menu.get_menu_tree(query).await?;
    Ok(R(ApiR::success(tree)))
}
