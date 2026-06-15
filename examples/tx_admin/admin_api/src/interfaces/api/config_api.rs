//! 配置管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::config::app_service::ConfigAppService;
use admin_proto::{CreateConfigRequest, UpdateConfigRequest, ListConfigsRequest, ConfigResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_config))
        .route("/{config_id}", get(get_config))
        .route("/{config_id}", put(update_config))
        .route("/{config_id}", delete(delete_config))
        .route("/list", post(list_configs))
        .route("/key/{key}", get(get_config_by_key))
}

fn map_config(c: admin_app::config::dto::ConfigResponse) -> ConfigResponse { ConfigResponse { id: c.id, category: c.category, config_type: c.config_type, name: c.name, config_key: c.config_key, value: c.value, visible: c.visible, remark: c.remark } }

async fn create_config(
    DiComp(config): DiComp<ConfigAppService>,
    Json(req): Json<CreateConfigRequest>,
) -> Result<R<ConfigResponse>, ApiErr> {
    ensure_permission("config:create").await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::config::dto::CreateConfigCommand { category: req.category, config_type: req.config_type, name: req.name, config_key: req.config_key, value: req.value, remark: opt_filter(req.remark) };
    let r = config.create_config(cmd, None).await?;
    Ok(R(ApiR::success(map_config(r))))
}

async fn get_config(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> Result<R<ConfigResponse>, ApiErr> {
    ensure_permission("config:view").await?;
    let r = config.get_config(config_id).await?;
    Ok(R(ApiR::success(map_config(r))))
}

async fn update_config(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<R<ConfigResponse>, ApiErr> {
    ensure_permission("config:update").await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::config::dto::UpdateConfigCommand { config_id, category: req.category, config_type: req.config_type, name: req.name, config_key: req.config_key, value: req.value, visible: req.visible, remark: opt_filter(req.remark) };
    let r = config.update_config(cmd, None).await?;
    Ok(R(ApiR::success(map_config(r))))
}

async fn delete_config(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("config:delete").await?;
    config.delete_config(config_id, None).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

async fn list_configs(
    DiComp(config): DiComp<ConfigAppService>,
    Json(req): Json<ListConfigsRequest>,
) -> Result<R<Page<ConfigResponse>>, ApiErr> {
    ensure_permission("config:view").await?;
    let query = admin_app::config::dto::ConfigQueryRequest { name: req.name, category: req.category, config_key: req.config_key, config_type: req.config_type, page: req.page, size: req.page_size };
    let page = config.get_config_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_config).collect(), page.page, page.size, page.total))))
}

/// GET /api/config/key/{key}
async fn get_config_by_key(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<R<ConfigResponse>, ApiErr> {
    ensure_permission("config:view").await?;
    let r = config.get_by_key(&key).await?;
    Ok(R(ApiR::success(map_config(r))))
}
