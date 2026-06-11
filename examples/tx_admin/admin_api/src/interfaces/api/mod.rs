use std::sync::Arc;
use axum::Router;
use tx_di_core::App;

/// 所有的路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
}