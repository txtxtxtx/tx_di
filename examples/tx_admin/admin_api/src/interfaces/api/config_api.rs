//! 配置管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::config::app_service::ConfigAppService;
use admin_proto::{CreateConfigRequest, UpdateConfigRequest, ListConfigsRequest, ConfigResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};

pub fn router() -> Router {
    Router::new()
        .api_route("/", post(create_config))
        .api_route("/{config_id}", get(get_config))
        .api_route("/{config_id}", put(update_config))
        .api_route("/{config_id}", delete(delete_config))
        .api_route("/list", post(list_configs))
}

fn map_config(c: admin_app::config::dto::ConfigResponse) -> ConfigResponse { ConfigResponse { id: c.id, category: c.category, config_type: c.config_type, name: c.name, config_key: c.config_key, value: c.value, visible: c.visible, remark: c.remark } }

async fn create_config(DiComp(config): DiComp<ConfigAppService>, Json(req): Json<CreateConfigRequest>) -> R<ConfigResponse> {
    let cmd = admin_app::config::dto::CreateConfigCommand { category: req.category, config_type: req.config_type, name: req.name, config_key: req.config_key, value: req.value, remark: req.remark };
    match config.create_config(cmd, None).await { Ok(r) => R(ApiR::success(map_config(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn get_config(DiComp(config): DiComp<ConfigAppService>, axum::extract::Path(config_id): axum::extract::Path<u64>) -> R<ConfigResponse> {
    match config.get_config(config_id).await { Ok(r) => R(ApiR::success(map_config(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn update_config(DiComp(config): DiComp<ConfigAppService>, axum::extract::Path(config_id): axum::extract::Path<u64>, Json(req): Json<UpdateConfigRequest>) -> R<ConfigResponse> {
    let cmd = admin_app::config::dto::UpdateConfigCommand { config_id, category: req.category, config_type: req.config_type, name: req.name, config_key: req.config_key, value: req.value, visible: req.visible, remark: req.remark };
    match config.update_config(cmd, None).await { Ok(r) => R(ApiR::success(map_config(r))), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn delete_config(DiComp(config): DiComp<ConfigAppService>, axum::extract::Path(config_id): axum::extract::Path<u64>) -> R<Empty> {
    match config.delete_config(config_id, None).await { Ok(()) => R(ApiRes::ok().into_typed()), Err(e) => R(ApiRes::from(e).into_typed()) }
}

async fn list_configs(DiComp(config): DiComp<ConfigAppService>, Json(req): Json<ListConfigsRequest>) -> R<Page<ConfigResponse>> {
    let query = admin_app::config::dto::ConfigQueryRequest { name: req.name, category: req.category, config_key: req.config_key, config_type: req.config_type, page: req.page, size: req.page_size };
    match config.get_config_page(query).await { Ok(page) => R(ApiR::success(Page::new(page.list.into_iter().map(map_config).collect(), page.page, page.size, page.total))), Err(e) => R(ApiRes::from(e).into_typed()) }
}
