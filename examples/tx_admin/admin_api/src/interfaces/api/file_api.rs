//! 文件管理 HTTP API
//!
//! - `POST /api/file/upload`  流式多文件上传（multipart/form-data）
//! - `GET  /api/file/{id}`    获取文件元数据
//! - `DELETE /api/file/{id}`  删除物理文件 + DB 软删除
//! - `GET  /api/file/{id}/download`  流式下载文件二进制
//! - `POST /api/file/list`    分页查询

use axum::Json;
use axum::extract::{Multipart, Path};
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use tx_di_axum::Router;
use axum::routing::{get, post, delete};
use tx_di_axum::bound::DiComp;
use admin_app::file::app_service::FileAppService;
use admin_proto::{ListFilesRequest, FileResponse, Empty};
use tx_common::{ApiR, ApiRes, Page};
use tx_error::AppError;
use tx_di_file::storage::FileStorageErr;
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;
use tokio_util::io::ReaderStream;
use futures::StreamExt;

pub fn router() -> Router {
    Router::new()
        .route("/upload", post(upload_files))
        .route("/{file_id}", get(get_file))
        .route("/{file_id}", delete(delete_file))
        .route("/{file_id}/download", get(download_file))
        .route("/list", post(list_files))
}

// ============================================================================
// 流式多文件上传
// ============================================================================

/// `POST /api/file/upload`
///
/// 接收 `multipart/form-data`：
/// - `config_id` (可选) — 公共配置 ID，应用于所有文件
/// - `file` (可多个) — 文件二进制，每个 file field 一个文件
///
/// 每个文件边收边写，全程零内存缓冲。
async fn upload_files(
    DiComp(file_svc): DiComp<FileAppService>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<ApiR<Vec<FileResponse>>, ApiErr> {
    // ── 提前检查 Content-Length，避免接收大文件到一半才发现超限 ──
    let max_size = file_svc.max_file_size();
    if max_size > 0 {
        if let Some(cl) = headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
        {
            if cl > max_size {
                return Err(AppError::from_code(FileStorageErr::FileTooLarge).into());
            }
        }
    }

    ensure_permission("file:upload").await?;
    let creator = StpUtil::get_login_id_as_string().await?;

    let mut config_id: Option<i32> = None;
    let mut results: Vec<FileResponse> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("length limit") {
            AppError::from_code(FileStorageErr::FileTooLarge)
        } else {
            AppError::from_anyhow(anyhow::anyhow!(e))
        }
    })? {
        match field.name() {
            Some("config_id") => {
                let text = field.text().await.map_err(|e| anyhow::anyhow!(e))?;
                config_id = text.parse().ok();
            }
            Some("file") => {
                let filename = field
                    .file_name()
                    .unwrap_or("unknown")
                    .to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();

                // ═══ multipart Stream → AsyncRead → write_stream ═══
                // 文件二进制从头到尾不进入应用内存
                let byte_stream = field.map(|r| {
                    r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                });
                let mut reader = tokio_util::io::StreamReader::new(byte_stream);

                let r = file_svc
                    .upload_file_stream(
                        filename,
                        content_type,
                        &mut reader,
                        config_id,
                        Some(creator.clone()),
                    )
                    .await?;
                results.push(r);
            }
            _ => { /* 忽略未知字段 */ }
        }
    }

    Ok(ApiR::success(results))
}

// ============================================================================
// 获取文件元数据
// ============================================================================

async fn get_file(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(file_id): Path<u64>,
) -> Result<ApiR<FileResponse>, ApiErr> {
    ensure_permission("file:view").await?;
    let r = file_svc.get_file(file_id).await?;
    Ok(ApiR::success(r))
}

// ============================================================================
// 删除文件（物理文件 + DB 软删除）
// ============================================================================

async fn delete_file(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(file_id): Path<u64>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("file:delete").await?;
    let login_id = StpUtil::get_login_id_as_string().await?;
    file_svc.delete_file(file_id, Some(login_id)).await?;
    Ok(ApiRes::ok().into_typed())
}

// ============================================================================
// 流式下载
// ============================================================================

/// `GET /api/file/{id}/download`
///
/// 流式返回文件二进制，设置 Content-Disposition / Content-Type / Content-Length。
/// 不经过 JSON 包装，不缓冲文件到内存。
async fn download_file(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(file_id): Path<u64>,
) -> Result<impl IntoResponse, ApiErr> {
    ensure_permission("file:download").await?;
    let stream = file_svc.download_file_stream(file_id).await?;

    let body = axum::body::Body::from_stream(ReaderStream::new(stream.reader));

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, &stream.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", stream.filename),
        )
        .header(header::CONTENT_LENGTH, stream.size)
        .body(body)
        .unwrap())
}

// ============================================================================
// 分页查询
// ============================================================================

async fn list_files(
    DiComp(file_svc): DiComp<FileAppService>,
    Json(req): Json<ListFilesRequest>,
) -> Result<ApiR<Page<FileResponse>>, ApiErr> {
    ensure_permission("file:view").await?;
    let page = file_svc.get_file_page(req).await?;
    Ok(ApiR::success(page))
}
