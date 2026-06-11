//! 文件管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, delete}};
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{
    UploadFileRequest, ListFilesRequest,
    FileResponse, Empty,
};
use crate::interfaces::dto::{ApiResponse, PageResponse};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/{file_id}", get(get_file))
        .route("/{file_id}", delete(delete_file))
        .route("/list", post(list_files))
        .with_state(app)
}

/// POST /api/file/upload
async fn upload_file(
    State(_app): State<Arc<App>>,
    Json(req): Json<UploadFileRequest>,
) -> Result<Json<ApiResponse<FileResponse>>, tx_error::AppError> {
    // TODO: 调用 FileAppService::upload
    let resp = FileResponse {
        id: 1,
        config_id: req.config_id,
        name: req.name.clone(),
        path: req.path.clone(),
        url: req.url.clone(),
        file_type: req.file_type.clone(),
        size: req.size,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// GET /api/file/{file_id}
async fn get_file(
    State(_app): State<Arc<App>>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<FileResponse>>, tx_error::AppError> {
    // TODO: 调用 FileAppService::get_by_id
    let resp = FileResponse {
        id: file_id,
        config_id: None,
        name: "placeholder".into(),
        path: String::new(),
        url: String::new(),
        file_type: None,
        size: 0,
    };
    Ok(Json(ApiResponse::success(resp)))
}

/// DELETE /api/file/{file_id}
async fn delete_file(
    State(_app): State<Arc<App>>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<Json<ApiResponse<Empty>>, tx_error::AppError> {
    // TODO: 调用 FileAppService::delete
    let _ = file_id;
    Ok(Json(ApiResponse::success(Empty {})))
}

/// POST /api/file/list
async fn list_files(
    State(_app): State<Arc<App>>,
    Json(req): Json<ListFilesRequest>,
) -> Result<Json<ApiResponse<PageResponse<FileResponse>>>, tx_error::AppError> {
    // TODO: 调用 FileAppService::list
    let page = PageResponse {
        list: vec![],
        total: 0,
        page: req.page,
        size: req.page_size,
    };
    Ok(Json(ApiResponse::success(page)))
}
