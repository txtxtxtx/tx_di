//! 权限管理 API

use axum::{Json, Router, extract::State, routing::get};
use std::sync::Arc;
use tx_di_core::App;

use crate::application::permission::PermissionService;
use crate::interfaces::dto::ApiResponse;

/// 权限路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/menu-tree", get(menu_tree))
        .route("/all", get(list_all))
        .with_state(app)
}

/// 获取菜单树
async fn menu_tree(
    State(app): State<Arc<App>>,
) -> Json<ApiResponse<Vec<crate::application::permission::PermissionTreeNode>>> {
    let service = app.inject::<PermissionService>();

    match service.get_menu_tree(None).await {
        Ok(tree) => Json(ApiResponse::success(tree)),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}

/// 获取所有权限列表
async fn list_all(
    State(app): State<Arc<App>>,
) -> Json<ApiResponse<Vec<crate::domain::permission::Permission>>> {
    let service = app.inject::<PermissionService>();

    match service.list_all(None).await {
        Ok(perms) => Json(ApiResponse::success(perms)),
        Err(e) => Json(ApiResponse::error(500, e.to_string())),
    }
}
