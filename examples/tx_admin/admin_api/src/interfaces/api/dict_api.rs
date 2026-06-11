//! 字典管理 HTTP API
//!
//! 包含字典类型和字典数据两部分。

use axum::{Json, Router, extract::State, routing::{get, post, put, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    CreateDictTypeRequest, UpdateDictTypeRequest, DeleteDictTypeRequest, GetDictTypeRequest,
    ListDictTypesRequest, DictTypeResponse,
    CreateDictDataRequest, UpdateDictDataRequest, DeleteDictDataRequest, GetDictDataRequest,
    ListDictDataRequest, DictDataResponse, Empty,
};
use crate::interfaces::dto::{ApiResponse, PageResponse};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // ── 字典类型 ──
        .route("/type", post(create_dict_type))
        .route("/type/{id}", get(get_dict_type))
        .route("/type/{id}", put(update_dict_type))
        .route("/type/{id}", delete(delete_dict_type))
        .route("/type/list", post(list_dict_types))
        // ── 字典数据 ──
        .route("/data", post(create_dict_data))
        .route("/data/{id}", get(get_dict_data))
        .route("/data/{id}", put(update_dict_data))
        .route("/data/{id}", delete(delete_dict_data))
        .route("/data/list", post(list_dict_data))
        .with_state(app)
}

// ══════════════════════════════════════════════════════════════
// 字典类型
// ══════════════════════════════════════════════════════════════

/// POST /api/dict/type
async fn create_dict_type(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateDictTypeRequest>,
) -> Result<Json<ApiResponse<DictTypeResponse>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::create_type
    let resp = DictTypeResponse {
        id: 1,
        name: req.name.clone(),
        dict_type: req.dict_type.clone(),
        status: 1,
        remark: req.remark.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/dict/type/{id}
async fn get_dict_type(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<DictTypeResponse>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::get_type_by_id
    let resp = DictTypeResponse {
        id,
        name: "placeholder".into(),
        dict_type: "placeholder".into(),
        status: 1,
        remark: None,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/dict/type/{id}
async fn update_dict_type(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateDictTypeRequest>,
) -> Result<Json<ApiResponse<DictTypeResponse>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::update_type
    req.id = id;
    let resp = DictTypeResponse {
        id,
        name: req.name.clone(),
        dict_type: req.dict_type.clone(),
        status: 1,
        remark: req.remark.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/dict/type/{id}
async fn delete_dict_type(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::delete_type
    let _ = id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/dict/type/list
async fn list_dict_types(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListDictTypesRequest>,
) -> Result<Json<ApiResponse<PageResponse<DictTypeResponse>>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::list_types
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}

// ══════════════════════════════════════════════════════════════
// 字典数据
// ══════════════════════════════════════════════════════════════

/// POST /api/dict/data
async fn create_dict_data(
    State(_app): State<Arc<App>>,
    Json(req): Json<CreateDictDataRequest>,
) -> Result<Json<ApiResponse<DictDataResponse>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::create_data
    let resp = DictDataResponse {
        id: 1,
        sort: req.sort,
        label: req.label.clone(),
        value: req.value.clone(),
        dict_type: req.dict_type.clone(),
        status: 1,
        color_type: req.color_type.clone(),
        css_class: req.css_class.clone(),
        remark: req.remark.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/dict/data/{id}
async fn get_dict_data(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<DictDataResponse>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::get_data_by_id
    let resp = DictDataResponse {
        id,
        sort: 0,
        label: "placeholder".into(),
        value: "placeholder".into(),
        dict_type: String::new(),
        status: 1,
        color_type: None,
        css_class: None,
        remark: None,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// PUT /api/dict/data/{id}
async fn update_dict_data(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut req): Json<UpdateDictDataRequest>,
) -> Result<Json<ApiResponse<DictDataResponse>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::update_data
    req.id = id;
    let resp = DictDataResponse {
        id,
        sort: req.sort,
        label: req.label.clone(),
        value: req.value.clone(),
        dict_type: req.dict_type.clone(),
        status: 1,
        color_type: req.color_type.clone(),
        css_class: req.css_class.clone(),
        remark: req.remark.clone(),
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/dict/data/{id}
async fn delete_dict_data(
    State(_app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::delete_data
    let _ = id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/dict/data/list
async fn list_dict_data(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListDictDataRequest>,
) -> Result<Json<ApiResponse<PageResponse<DictDataResponse>>>, tx_error::AppError> {
    // TODO: 调用 DictAppService::list_data
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}
