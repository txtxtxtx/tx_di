//! 文件管理 HTTP API

use axum::Json;
use tx_di_axum::Router;
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

async fn upload_file(
    DiComp(file_svc): DiComp<FileAppService>,
    Json(req): Json<UploadFileRequest>,
) -> Result<ApiR<FileResponse>, ApiErr> {
    ensure_permission("file:upload").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    let r = file_svc.upload_file(req, Some(login_id)).await?;
    Ok(ApiR::success(r))
}

async fn get_file(
    DiComp(file_svc): DiComp<FileAppService>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<ApiR<FileResponse>, ApiErr> {
    ensure_permission("file:view").await?;
    let r = file_svc.get_file(file_id).await?;
    Ok(ApiR::success(r))
}

async fn delete_file(
    DiComp(file_svc): DiComp<FileAppService>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("file:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    file_svc.delete_file(file_id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

async fn list_files(
    DiComp(file_svc): DiComp<FileAppService>,
    Json(req): Json<ListFilesRequest>,
) -> Result<ApiR<Page<FileResponse>>, ApiErr> {
    ensure_permission("file:view").await?;
    let page = file_svc.get_file_page(req).await?;
    Ok(ApiR::success(page))
}

async fn download_file(
    DiComp(file_svc): DiComp<FileAppService>,
    axum::extract::Path(file_id): axum::extract::Path<u64>,
) -> Result<ApiR<DownloadFileResponse>, ApiErr> {
    ensure_permission("file:download").await?;
    let r = file_svc.download_file(file_id).await?;
    Ok(ApiR::success(r))
}
