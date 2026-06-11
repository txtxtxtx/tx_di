//! 文件管理 API
//!
//! Handler 只负责 HTTP 协议转换。

use axum::{Json, Router, extract::{Path, Query, State}, routing::{delete, get, post, put}};
use std::sync::Arc;
use tx_di_core::App;

use crate::domain::{AppError, AdminErr};
use crate::domain::file::{FileRepository};
use crate::domain::file::repo::ToastyFileRepository;
use tx_common::{ApiR, ApiRes, Page};
use crate::interfaces::dto::common::PageQuery;
use crate::interfaces::dto::file_dto::{FileDto, CreateFileRequest, UpdateFileRequest};

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(list_files).post(create_file))
        .route("/{id}", get(get_file).put(update_file).delete(delete_file))
        .with_state(app)
}

async fn list_files(
    State(app): State<Arc<App>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiR<Page<FileDto>>>, AppError> {
    let repo = app.inject::<ToastyFileRepository>();
    let (files, total) = repo.find_page(query.page as u64, query.size as u64).await?;
    let dtos: Vec<FileDto> = files.iter().map(FileDto::from).collect();
    Ok(Json(ApiR::success(Page::new(dtos, query.page, query.size, total as i64))))
}

async fn get_file(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiR<FileDto>>, AppError> {
    let repo = app.inject::<ToastyFileRepository>();
    let file = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::FileNotFound, id.to_string()))?;
    Ok(Json(ApiR::success(FileDto::from(&file))))
}

async fn create_file(
    State(app): State<Arc<App>>,
    Json(req): Json<CreateFileRequest>,
) -> Result<Json<ApiR<FileDto>>, AppError> {
    let repo = app.inject::<ToastyFileRepository>();
    let file = crate::domain::file::File {
        id: 0,
        config_id: req.config_id,
        name: req.name,
        file_path: req.file_path,
        url: req.url,
        file_type: req.file_type,
        size: req.size.unwrap_or(0),
        creator: None,
        updater: None,
        created_at: jiff::Timestamp::now(),
        updated_at: jiff::Timestamp::now(),
        deleted: 0,
    };
    repo.save(&file).await?;
    Ok(Json(ApiR::success(FileDto::from(&file))))
}

async fn update_file(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateFileRequest>,
) -> Result<Json<ApiR<FileDto>>, AppError> {
    let repo = app.inject::<ToastyFileRepository>();
    let mut file = repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::FileNotFound, id.to_string()))?;
    if let Some(v) = req.config_id { file.config_id = Some(v); }
    if let Some(v) = req.name { file.name = Some(v); }
    if let Some(v) = req.file_path { file.file_path = v; }
    if let Some(v) = req.url { file.url = v; }
    if let Some(v) = req.file_type { file.file_type = Some(v); }
    if let Some(v) = req.size { file.size = v; }
    repo.save(&file).await?;
    Ok(Json(ApiR::success(FileDto::from(&file))))
}

async fn delete_file(
    State(app): State<Arc<App>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiRes>, AppError> {
    let repo = app.inject::<ToastyFileRepository>();
    repo.find_by_id(id).await?.ok_or(AppError::with_context(AdminErr::FileNotFound, id.to_string()))?;
    repo.delete(id).await?;
    Ok(Json(ApiRes::ok()))
}
