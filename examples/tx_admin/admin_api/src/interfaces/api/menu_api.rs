//! 菜单管理 HTTP API

use axum::Json;
use tx_di_sa_token::StpUtil;
use tx_di_axum::Router;
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

async fn create_menu(
    DiComp(menu): DiComp<MenuAppService>,
    Json(mut req): Json<CreateMenuRequest>,
) -> Result<ApiR<MenuResponse>, ApiErr> {
    ensure_permission("menu:create").await?;
    use admin_app::empty_string::opt_filter;
    req.path = opt_filter(req.path);
    req.icon = opt_filter(req.icon);
    req.component = opt_filter(req.component);
    req.component_name = opt_filter(req.component_name);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = menu.create_menu(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn get_menu(
    DiComp(menu): DiComp<MenuAppService>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> Result<ApiR<MenuResponse>, ApiErr> {
    ensure_permission("menu:view").await?;
    let query = ListMenusRequest { name: None, status: None, types: None };
    let list = menu.get_menu_list(query).await?;
    let r = list.into_iter().find(|m| m.id == menu_id)
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(ApiR::success(r))
}

async fn update_menu(
    DiComp(menu): DiComp<MenuAppService>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateMenuRequest>,
) -> Result<ApiR<MenuResponse>, ApiErr> {
    ensure_permission("menu:update").await?;
    use admin_app::empty_string::opt_filter;
    req.menu_id = menu_id;
    req.path = opt_filter(req.path);
    req.icon = opt_filter(req.icon);
    req.component = opt_filter(req.component);
    req.component_name = opt_filter(req.component_name);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = menu.update_menu(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn delete_menu(
    DiComp(menu): DiComp<MenuAppService>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("menu:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    menu.delete_menu(menu_id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_menus(
    DiComp(menu): DiComp<MenuAppService>,
    Json(mut req): Json<ListMenusRequest>,
) -> Result<ApiR<Vec<MenuTreeNode>>, ApiErr> {
    ensure_permission("menu:view").await?;
    req.types = None;
    let tree = menu.get_menu_tree(req).await?;
    Ok(ApiR::success(tree))
}
