//! 配置管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateConfigRequest, UpdateConfigRequest, DeleteConfigRequest, GetConfigRequest,
    ListConfigsRequest, ConfigResponse, Empty,
};
use crate::interfaces::dto::{ApiResponse, PageResponse};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", post(create_config))
        .route("/{config_id}", get(get_config))
        .route("/{config_id}", put(update_config))
        .route("/{config_id}", delete(delete_config))
        .route("/list", post(list_configs))
        .with_state(app)
}

/// POST /api/config/
async fn create_config(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigResponse>>, tx_error::AppError> {
    // TODO: 调用 ConfigAppService::create
    let resp = ConfigResponse {
        id: 1,
        category: req.category.clone(),
        config_type: req.config_type,
        name: req.name.clone(),
        config_key: req.config_key.clone(),
        value: req.value.clone(),
        visible: 0,
        remark: req.remark.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/config/{config_id}
async fn get_config(
    State(_app): State<Arc<App>>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<ConfigResponse>>, tx_error::AppError> {
    // TODO: 调用 ConfigAppService::get_by_id
    let resp = ConfigResponse {
        id: config_id,
        category: String::new(),
        config_type: 0,
        name: "placeholder".into(),
        config_key: String::new(),
        value: String::new(),
        visible: 0,
        remark: None,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/config/{config_id}
async fn update_config(
    State(_app): State<Arc<App>>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigResponse>>, tx_error::AppError> {
    // TODO: 调用 ConfigAppService::update
    req.config_id = config_id;
    let resp = ConfigResponse {
        id: config_id,
        category: req.category.clone(),
        config_type: req.config_type,
        name: req.name.clone(),
        config_key: req.config_key.clone(),
        value: req.value.clone(),
        visible: req.visible,
        remark: req.remark.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/config/{config_id}
async fn delete_config(
    State(_app): State<Arc<App>>,
    axum::extract::Path(config_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 ConfigAppService::delete
    let _ = config_id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/config/list
async fn list_configs(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListConfigsRequest>,
) -> Result<Json<ApiResponse<PageResponse<ConfigResponse>>>, tx_error::AppError> {
    // TODO: 调用 ConfigAppService::list
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}
