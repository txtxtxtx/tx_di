//! 文件管理 HTTP API

use axum::{Json, Router, routing::{get, post, delete}};
use axum::response::IntoResponse;
use tx_di_axum::bound::DiComp;
use admin_app::file::app_service::FileAppService;
use admin_proto::{UploadFileRequest, ListFilesRequest, FileResponse};
use tx_common::{ApiR, ApiRes, Page};

pub fn router() -> Router {
    Router::new()
        .route("/", post(upload_file))
        .route("/{file_id}", get(get_file))
        .route("/{file_id}", delete(delete_file))
        .route("/list", post(list_files))
}

fn map_file(f: admin_app::file::dto::FileResponse) -> FileResponse { FileResponse { id: f.id, config_id: f.config_id, name: f.name, path: f.path, url: f.url, file_type: f.file_type, size: f.size } }

async fn upload_file(DiComp(file_svc): DiComp<FileAppService>, Json(req): Json<UploadFileRequest>) -> impl IntoResponse {
    let cmd = admin_app::file::dto::UploadFileCommand { name: req.name, path: req.path, url: req.url, file_type: req.file_type, size: req.size, config_id: req.config_id };
    match file_svc.upload_file(cmd, None).await { Ok(r) => ApiR::success(map_file(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn get_file(DiComp(file_svc): DiComp<FileAppService>, axum::extract::Path(file_id): axum::extract::Path<u64>) -> impl IntoResponse {
    match file_svc.get_file(file_id).await { Ok(r) => ApiR::success(map_file(r)), Err(e) => ApiRes::from(e).into_typed() }
}

async fn delete_file(DiComp(file_svc): DiComp<FileAppService>, axum::extract::Path(file_id): axum::extract::Path<u64>) -> impl IntoResponse {
    match file_svc.delete_file(file_id, None).await { Ok(()) => ApiRes::ok(), Err(e) => ApiRes::from(e) }
}

async fn list_files(DiComp(file_svc): DiComp<FileAppService>, Json(req): Json<ListFilesRequest>) -> impl IntoResponse {
    let query = admin_app::file::dto::FileQueryRequest { name: req.name, file_type: req.file_type, config_id: req.config_id, page: req.page, size: req.page_size };
    match file_svc.get_file_page(query).await { Ok(page) => ApiR::success(Page::new(page.list.into_iter().map(map_file).collect(), page.page, page.size, page.total)), Err(e) => ApiRes::from(e).into_typed() }
}
