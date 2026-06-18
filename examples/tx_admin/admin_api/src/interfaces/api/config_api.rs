//! 配置管理 HTTP API

use axum::Json;
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::config::app_service::ConfigAppService;
use admin_proto::{CreateConfigRequest, UpdateConfigRequest, ListConfigsRequest, ConfigResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_config))
        .route("/{config_id}", get(get_config))
        .route("/{config_id}", put(update_config))
        .route("/{config_id}", delete(delete_config))
        .route("/list", post(list_configs))
        .route("/key/{key}", get(get_config_by_key))
}

async fn create_config(
    DiComp(config): DiComp<ConfigAppService>,
    Json(req): Json<CreateConfigRequest>,
) -> Result<ApiR<ConfigResponse>, ApiErr> {
    ensure_permission("config:create").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = config.create_config(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn get_config(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> Result<ApiR<ConfigResponse>, ApiErr> {
    ensure_permission("config:view").await?;
    let r = config.get_config(config_id).await?;
    Ok(ApiR::success(r))
}

async fn update_config(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<ApiR<ConfigResponse>, ApiErr> {
    ensure_permission("config:update").await?;
    use admin_app::empty_string::opt_filter;
    let mut req = req;
    req.config_id = config_id;
    req.remark = opt_filter(req.remark);
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = config.update_config(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn delete_config(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("config:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    config.delete_config(config_id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_configs(
    DiComp(config): DiComp<ConfigAppService>,
    Json(req): Json<ListConfigsRequest>,
) -> Result<ApiR<Page<ConfigResponse>>, ApiErr> {
    ensure_permission("config:view").await?;
    let page = config.get_config_page(req).await?;
    Ok(ApiR::success(page))
}

/// GET /api/config/key/{key}
async fn get_config_by_key(
    DiComp(config): DiComp<ConfigAppService>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<ApiR<ConfigResponse>, ApiErr> {
    ensure_permission("config:view").await?;
    let r = config.get_by_key(&key).await?;
    Ok(ApiR::success(r))
}
