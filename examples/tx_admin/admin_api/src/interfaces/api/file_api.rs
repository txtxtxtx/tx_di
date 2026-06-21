//! 文件管理 HTTP API
//!
//! 文件操作：
//! - `POST /api/file/upload`    流式多文件上传
//! - `GET  /api/file/{id}`      获取文件元数据
//! - `DELETE /api/file/{id}`    删除物理文件 + DB 软删除
//! - `GET  /api/file/{id}/download`  流式下载
//! - `POST /api/file/list`      分页查询
//!
//! 预览：
//! - `GET  /api/file/pre/url/{id}`     获取预览地址
//! - `GET  /api/file/pre/serve/{*path}` 本地文件静态服务
//!
//! 文件配置：
//! - `GET  /api/file/config/list`       配置列表
//! - `GET  /api/file/config/{id}`       配置详情
//! - `POST /api/file/config`            新增配置
//! - `PUT  /api/file/config/{id}`       修改配置
//! - `DELETE /api/file/config/{id}`     删除配置
//! - `PUT  /api/file/config/{id}/master` 设为主配置

use axum::Json;
use axum::extract::{Multipart, Path};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use tx_di_axum::Router;
use axum::routing::{get, post, put, delete};
use tx_di_axum::bound::DiComp;
use admin_app::file::app_service::FileAppService;
use admin_proto::{
    ListFilesRequest, FileResponse, FileConfigResponse, Empty,
    CreateFileConfigRequest, UpdateFileConfigRequest,
};
use tx_common::{ApiR, ApiRes, Page};
use tx_error::AppError;
use tx_di_core::CodeMsg;
use tx_di_file::storage::FileStorageErr;
use tx_di_axum::BodySizeLimitLayer;
use crate::auth::ensure_permission;
use crate::error::ApiErr;
use tx_di_sa_token::StpUtil;
use tokio_util::io::ReaderStream;
use futures::StreamExt;

/// `max_body_size`: 全局请求体上限（字节），用于 Content-Length 提前拦截。0 表示不限制。
pub fn router(max_body_size: u64) -> Router {
    // 文件操作路由
    let file_routes = Router::new()
        .route(
            "/upload",
            post(upload_files)
                .route_layer(BodySizeLimitLayer::new(
                    max_body_size,
                    FileStorageErr::FileTooLarge.code(),
                    FileStorageErr::FileTooLarge.message(),
                )),
        )
        .route("/{file_id}", get(get_file))
        .route("/{file_id}", delete(delete_file))
        .route("/{file_id}/download", get(download_file))
        .route("/list", post(list_files));

    // 预览路由（需鉴权）
    let pre_routes = Router::new()
        .route("/url/{file_id}", get(get_preview_url));

    // 文件配置路由
    let config_routes = Router::new()
        .route("/list", get(list_configs))
        .route("/{id}", get(get_config))
        .route("/", post(create_config))
        .route("/{id}", put(update_config))
        .route("/{id}", delete(delete_config))
        .route("/{id}/master", put(set_master_config));

    file_routes.nest("/pre", pre_routes).nest("/config", config_routes)
}

/// 公开路由：无需鉴权（URL 含 UUID，不可猜测）
///
/// 当前包含：
/// - `GET /api/file/pre/serve/{*path}` 本地文件静态服务
pub fn open_router() -> Router {
    Router::new().route("/api/file/pre/serve/{*path}", get(serve_local_file))
}

// ============================================================================
// 流式多文件上传
// ============================================================================

/// `POST /api/file/upload`
async fn upload_files(
    DiComp(file_svc): DiComp<FileAppService>,
    mut multipart: Multipart,
) -> Result<ApiR<Vec<FileResponse>>, ApiErr> {
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

// ============================================================================
// 文件配置 CRUD
// ============================================================================

/// `GET /api/file/config/list`
async fn list_configs(
    DiComp(file_svc): DiComp<FileAppService>,
) -> Result<ApiR<Vec<FileConfigResponse>>, ApiErr> {
    ensure_permission("file:view").await?;
    let configs = file_svc.get_config_all().await?;
    let resp = configs.into_iter().map(|c| {
        FileConfigResponse {
            id: c.id,
            name: c.name,
            storage: c.storage,
            remark: c.remark,
            master: c.master,
            config: c.config,
        }
    }).collect();
    Ok(ApiR::success(resp))
}

/// `GET /api/file/config/{id}`
async fn get_config(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(id): Path<i32>,
) -> Result<ApiR<FileConfigResponse>, ApiErr> {
    ensure_permission("file:view").await?;
    let c = file_svc.get_config(id).await?;
    Ok(ApiR::success(FileConfigResponse {
        id: c.id,
        name: c.name,
        storage: c.storage,
        remark: c.remark,
        master: c.master,
        config: c.config,
    }))
}

/// `POST /api/file/config`
async fn create_config(
    DiComp(file_svc): DiComp<FileAppService>,
    Json(req): Json<CreateFileConfigRequest>,
) -> Result<ApiR<FileConfigResponse>, ApiErr> {
    ensure_permission("file:upload").await?;
    let creator = StpUtil::get_login_id_as_string().await.ok();
    let c = file_svc
        .create_config(req.name, req.storage, req.remark, req.config, creator)
        .await?;
    Ok(ApiR::success(FileConfigResponse {
        id: c.id,
        name: c.name,
        storage: c.storage,
        remark: c.remark,
        master: c.master,
        config: c.config,
    }))
}

/// `PUT /api/file/config/{id}`
async fn update_config(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateFileConfigRequest>,
) -> Result<ApiR<FileConfigResponse>, ApiErr> {
    ensure_permission("file:upload").await?;
    let updater = StpUtil::get_login_id_as_string().await.ok();
    let c = file_svc
        .update_config(id, req.name, req.storage, req.remark, req.config, updater)
        .await?;
    Ok(ApiR::success(FileConfigResponse {
        id: c.id,
        name: c.name,
        storage: c.storage,
        remark: c.remark,
        master: c.master,
        config: c.config,
    }))
}

/// `DELETE /api/file/config/{id}`
async fn delete_config(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(id): Path<i32>,
) -> Result<ApiR<Empty>, ApiErr> {
    ensure_permission("file:delete").await?;
    let updater = StpUtil::get_login_id_as_string().await.ok();
    file_svc.delete_config(id, updater).await?;
    Ok(ApiRes::ok().into_typed())
}

/// `PUT /api/file/config/{id}/master`
async fn set_master_config(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(id): Path<i32>,
) -> Result<ApiR<FileConfigResponse>, ApiErr> {
    ensure_permission("file:upload").await?;
    let updater = StpUtil::get_login_id_as_string().await.ok();
    let c = file_svc.set_master_config(id, updater).await?;
    Ok(ApiR::success(FileConfigResponse {
        id: c.id,
        name: c.name,
        storage: c.storage,
        remark: c.remark,
        master: c.master,
        config: c.config,
    }))
}

// ============================================================================
// 预览 URL
// ============================================================================

/// `GET /api/file/pre/url/{file_id}`
async fn get_preview_url(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(file_id): Path<u64>,
) -> Result<ApiR<admin_app::file::dto::PreviewUrlResponse>, ApiErr> {
    ensure_permission("file:download").await?;
    let info = file_svc.get_preview_url(file_id).await?;
    Ok(ApiR::success(info))
}

// ============================================================================
// 本地文件静态服务（供预览 URL 使用，无鉴权——UUID 路径不可猜测）
// ============================================================================

/// `GET /api/file/pre/serve/{*path}`
async fn serve_local_file(
    DiComp(file_svc): DiComp<FileAppService>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, ApiErr> {
    let base = file_svc.serve_base_dir().await.unwrap_or_else(|| "./uploads".into());
    let safe_path = path.trim_start_matches('/');
    let full = std::path::PathBuf::from(&base).join(safe_path);

    // 防止路径穿越
    let canonical = full.canonicalize().unwrap_or(full.clone());
    let base_canonical = std::path::PathBuf::from(&base).canonicalize().unwrap_or(std::path::PathBuf::from(&base));
    if !canonical.starts_with(&base_canonical) {
        return Err(ApiErr::from(AppError::with_context(
            FileStorageErr::NotFound,
            "禁止的文件路径",
        )));
    }

    match tokio::fs::File::open(&canonical).await {
        Ok(file) => {
            let mime = tx_di_file::storage::guess_mime_type(&path);
            let metadata = file.metadata().await.unwrap();
            let body = axum::body::Body::from_stream(ReaderStream::new(tokio::io::BufReader::new(file)));
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .header(header::CONTENT_LENGTH, metadata.len())
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(body)
                .unwrap())
        }
        Err(_) => Err(ApiErr::from(AppError::from_code(FileStorageErr::NotFound))),
    }
}
