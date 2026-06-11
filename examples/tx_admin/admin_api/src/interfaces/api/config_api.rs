//! 配置管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{CreateConfigRequest, UpdateConfigRequest, ListConfigsRequest, ConfigResponse, Empty};
use crate::services;
use tx_common::{ApiR, ApiRes, Page};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_config))
        .route("/{config_id}", get(get_config))
        .route("/{config_id}", put(update_config))
        .route("/{config_id}", delete(delete_config))
        .route("/list", post(list_configs))
        .with_state(app)
}

fn map_config(c: admin_app::config::dto::ConfigResponse) -> ConfigResponse {
    ConfigResponse {
        id: c.id, category: c.category, config_type: c.config_type,
        name: c.name, config_key: c.config_key, value: c.value,
        visible: c.visible, remark: c.remark,
    }
}

async fn create_config(
    Json(req): Json<CreateConfigRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::config::dto::CreateConfigCommand {
        category: req.category, config_type: req.config_type,
        name: req.name, config_key: req.config_key,
        value: req.value, remark: req.remark,
    };
    match services::get().config.create_config(cmd, None).await {
        Ok(r) => ApiR::success(map_config(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_config(
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().config.get_config(config_id).await {
        Ok(r) => ApiR::success(map_config(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn update_config(
    axum::extract::Path(config_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::config::dto::UpdateConfigCommand {
        config_id, category: req.category, config_type: req.config_type,
        name: req.name, config_key: req.config_key, value: req.value,
        visible: req.visible, remark: req.remark,
    };
    match services::get().config.update_config(cmd, None).await {
        Ok(r) => ApiR::success(map_config(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn delete_config(
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().config.delete_config(config_id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_configs(
    Json(req): Json<ListConfigsRequest>,
) -> impl IntoResponse {
    let query = admin_app::config::dto::ConfigQueryRequest {
        name: req.name, category: req.category,
        config_key: req.config_key, config_type: req.config_type,
        page: req.page, size: req.page_size,
    };
    match services::get().config.get_config_page(query).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_config).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
