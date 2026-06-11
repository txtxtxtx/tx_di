//! 文件管理 HTTP API

use axum::{Json, Router, extract::State, routing::{get, post, delete}};
use axum::response::IntoResponse;
use std::sync::Arc;
use tx_di_core::App;

use admin_proto::{UploadFileRequest, ListFilesRequest, FileResponse, Empty};
use crate::services;
use tx_common::{ApiR, ApiRes, Page};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/{file_id}", get(get_file))
        .route("/{file_id}", delete(delete_file))
        .route("/list", post(list_files))
        .with_state(app)
}

fn map_file(f: admin_app::file::dto::FileResponse) -> FileResponse {
    FileResponse {
        id: f.id, config_id: f.config_id, name: f.name,
        path: f.path, url: f.url, file_type: f.file_type, size: f.size,
    }
}

async fn upload_file(
    Json(req): Json<UploadFileRequest>,
) -> impl IntoResponse {
    let cmd = admin_app::file::dto::UploadFileCommand {
        name: req.name, path: req.path, url: req.url,
        file_type: req.file_type, size: req.size, config_id: req.config_id,
    };
    match services::get().file.upload_file(cmd, None).await {
        Ok(r) => ApiR::success(map_file(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn get_file(
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().file.get_file(file_id).await {
        Ok(r) => ApiR::success(map_file(r)),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}

async fn delete_file(
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    match services::get().file.delete_file(file_id, None).await {
        Ok(()) => ApiRes::ok(),
        Err(e) => ApiRes::from(e),
    }
}

async fn list_files(
    Json(req): Json<ListFilesRequest>,
) -> impl IntoResponse {
    let query = admin_app::file::dto::FileQueryRequest {
        name: req.name, file_type: req.file_type,
        config_id: req.config_id, page: req.page, size: req.page_size,
    };
    match services::get().file.get_file_page(query).await {
        Ok(page) => ApiR::success(Page::new(
            page.list.into_iter().map(map_file).collect(),
            page.page, page.size, page.total,
        )),
        Err(e) => ApiRes::from(e).into_typed(),
    }
}
