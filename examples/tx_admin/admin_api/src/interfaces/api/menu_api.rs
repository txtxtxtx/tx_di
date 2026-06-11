//! 菜单管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateMenuRequest, UpdateMenuRequest, DeleteMenuRequest, GetMenuRequest,
    ListMenusRequest, MenuResponse, Empty,
};
use crate::interfaces::dto::ApiResponse;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_menu))
        .route("/{menu_id}", get(get_menu))
        .route("/{menu_id}", put(update_menu))
        .route("/{menu_id}", delete(delete_menu))
        .route("/list", post(list_menus))
        .with_state(app)
}

/// POST /api/menu/
async fn create_menu(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateMenuRequest>,
) -> Result<Json<ApiResponse<MenuResponse>>, tx_error::AppError> {
    // TODO: 调用 MenuAppService::create
    let resp = MenuResponse {
        id: 1,
        name: req.name.clone(),
        permission: req.permission.clone(),
        types: req.types,
        sort: req.sort,
        parent_id: req.parent_id,
        path: req.path.clone(),
        icon: req.icon.clone(),
        component: req.component.clone(),
        component_name: req.component_name.clone(),
        status: 1,
        visible: 1,
        keep_alive: 0,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/menu/{menu_id}
async fn get_menu(
    State(_app): State<Arc<App>>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<MenuResponse>>, tx_error::AppError> {
    // TODO: 调用 MenuAppService::get_by_id
    let resp = MenuResponse {
        id: menu_id,
        name: "placeholder".into(),
        permission: String::new(),
        types: 0,
        sort: 0,
        parent_id: 0,
        path: None,
        icon: None,
        component: None,
        component_name: None,
        status: 1,
        visible: 1,
        keep_alive: 0,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/menu/{menu_id}
async fn update_menu(
    State(_app): State<Arc<App>>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateMenuRequest>,
) -> Result<Json<ApiResponse<MenuResponse>>, tx_error::AppError> {
    // TODO: 调用 MenuAppService::update
    req.menu_id = menu_id;
    let resp = MenuResponse {
        id: menu_id,
        name: req.name.clone(),
        permission: req.permission.clone(),
        types: req.types,
        sort: req.sort,
        parent_id: req.parent_id,
        path: req.path.clone(),
        icon: req.icon.clone(),
        component: req.component.clone(),
        component_name: req.component_name.clone(),
        status: 1,
        visible: req.visible,
        keep_alive: req.keep_alive,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/menu/{menu_id}
async fn delete_menu(
    State(_app): State<Arc<App>>,
    axum::extract::Path(menu_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 MenuAppService::delete
    let _ = menu_id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/menu/list
async fn list_menus(
    State(_app): State<Arc<App>>,
    Json(_req): Json<ListMenusRequest>,
) -> Result<Json<ApiResponse<Vec<MenuResponse>>>, tx_error::AppError> {
    // TODO: 调用 MenuAppService::list (菜单为树形，不分页)
    Ok(Json(ApiResponse::success(vec![])))
}
