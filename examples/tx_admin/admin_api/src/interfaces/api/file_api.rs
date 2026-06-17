//! 文件管理 HTTP API

use axum::Json;
use tx_di_axum::{R, Router};
use axum::routing::{get, post, delete};
use tx_di_axum::bound::DiComp;
use admin_app::file::app_service::FileAppService;
use admin_proto::{UploadFileRequest, ListFilesRequest, FileResponse, DownloadFileResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;

pub fn router() -> Router {
    Router::new()
        .route("/", post(upload_file))
        .route("/{file_id}", get(get_file))
        .route("/{file_id}", delete(delete_file))
        .route("/{file_id}/download", get(download_file))
        .route("/list", post(list_files))
}

fn map_file(f: admin_app::file::dto::FileResponse) -> FileResponse { FileResponse { id: f.id, config_id: f.config_id, name: f.name, path: f.path, url: f.url, file_type: f.file_type, size: f.size } }

async fn upload_file(
    DiComp(file_svc): DiComp<FileAppService>,
    Json(req): Json<UploadFileRequest>,
) -> Result<R<FileResponse>, ApiErr> {
    ensure_permission("file:upload").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    use admin_app::empty_string::opt_filter;
    let cmd = admin_app::file::dto::UploadFileCommand { name: req.name, path: req.path, url: req.url, file_type: opt_filter(req.file_type), size: req.size, config_id: req.config_id };
    let r = file_svc.upload_file(cmd, Some(login_id)).await?;
    Ok(R(ApiR::success(map_file(r))))
}

async fn get_file(
    DiComp(file_svc): DiComp<FileAppService>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<R<FileResponse>, ApiErr> {
    ensure_permission("file:view").await?;
    let r = file_svc.get_file(file_id).await?;
    Ok(R(ApiR::success(map_file(r))))
}

async fn delete_file(
    DiComp(file_svc): DiComp<FileAppService>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<R<Empty>, ApiErr> {
    ensure_permission("file:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    file_svc.delete_file(file_id, Some(login_id)).await?;
    Ok(R(ApiRes::ok().into_typed()))
}

async fn list_files(
    DiComp(file_svc): DiComp<FileAppService>,
    Json(req): Json<ListFilesRequest>,
) -> Result<R<Page<FileResponse>>, ApiErr> {
    ensure_permission("file:view").await?;
    let query = admin_app::file::dto::FileQueryRequest { name: req.name, file_type: req.file_type, config_id: req.config_id, page: req.page, size: req.page_size };
    let page = file_svc.get_file_page(query).await?;
    Ok(R(ApiR::success(Page::new(page.list.into_iter().map(map_file).collect(), page.page, page.size, page.total))))
}

async fn download_file(
    DiComp(file_svc): DiComp<FileAppService>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<R<DownloadFileResponse>, ApiErr> {
    ensure_permission("file:download").await?;
    let r = file_svc.download_file(file_id).await?;
    Ok(R(ApiR::success(DownloadFileResponse { url: r.url, filename: r.filename, size: r.size as u64, content_type: r.content_type })))
}
