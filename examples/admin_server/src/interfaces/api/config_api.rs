//! 配置管理 API
//!
//! Handler 只负责 HTTP 协议转换，业务逻辑交给领域服务。

use axum::{Json, Router, extract::{Path, Query, State}, routing::{delete, get, post, put}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::{AppError, AdminErr};
use crate::domain::config::{ConfigRepository, ConfigType};
use crate::domain::config::repo::ToastyConfigRepository;
use crate::interfaces::dto::common::{ApiResponse, PageQuery, PageResponse};
use crate::interfaces::dto::config_dto::{ConfigDto, CreateConfigRequest, UpdateConfigRequest};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_configs).post(create_config))
        .route("/{id}", get(get_config).put(update_config).delete(delete_config))
        .with_state(app)
}

async fn list_configs(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiResponse<PageResponse<ConfigDto>>>, AppError> {
    let repo = app.inject::<ToastyConfigRepository>();
    let (configs, total) = repo.find_page(query.keyword.as_deref(), query.page, query.page_size).await?;
    let dtos: Vec<ConfigDto> = configs.iter().map(ConfigDto::from).collect();
    Ok(Json(ApiResponse::success(PageResponse::new(dtos, total, query.page, query.page_size))))
}

async fn get_config(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<ConfigDto>>, AppError> {
    let repo = app.inject::<ToastyConfigRepository>();
    let config = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::NotFound, id.to_string()))?;
    Ok(Json(ApiResponse::success(ConfigDto::from(&config))))
}

async fn create_config(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigDto>>, AppError> {
    let repo = app.inject::<ToastyConfigRepository>();
    // 检查 key 唯一性
    if repo.find_by_key(&req.config_key).await?.is_some() {
        return Err(AppError::with_context(AdminErr::Duplicate, format!("config_key '{}' 已存在", req.config_key)));
    }
    let config = crate::domain::config::Config {
        id: 0,
        category: req.category,
        config_type: ConfigType::Custom,
        name: req.name,
        config_key: req.config_key,
        value: req.value,
        visible: req.visible.unwrap_or(true),
        remark: req.remark,
        creator: None,
        updater: None,
        created_at: jiff::Timestamp::now(),
        updated_at: jiff::Timestamp::now(),
        deleted: 0,
    };
    repo.save(&config).await?;
    Ok(Json(ApiResponse::success(ConfigDto::from(&config))))
}

async fn update_config(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigDto>>, AppError> {
    let repo = app.inject::<ToastyConfigRepository>();
    let mut config = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::NotFound, id.to_string()))?;
    if let Some(v) = req.category { config.category = Some(v); }
    if let Some(v) = req.name { config.name = v; }
    if let Some(v) = req.config_key { config.config_key = v; }
    if let Some(v) = req.value { config.value = Some(v); }
    if let Some(v) = req.visible { config.visible = v; }
    if let Some(v) = req.remark { config.remark = Some(v); }
    repo.save(&config).await?;
    Ok(Json(ApiResponse::success(ConfigDto::from(&config))))
}

async fn delete_config(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let repo = app.inject::<ToastyConfigRepository>();
    repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::NotFound, id.to_string()))?;
    repo.delete(id).await?;
    Ok(Json(ApiResponse::<()>::ok()))
}
