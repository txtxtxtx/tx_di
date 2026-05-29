//! 文件服务 API
//!
//! 提供文件上传、下载、删除等接口。

use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post},
};
use std::sync::Arc;
use tx_di_core::App;

use crate::application::file::FileService;
use crate::interfaces::dto::ApiResponse;

/// 文件路由
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/download/{*path}", get(download_file))
        .route("/delete", delete(delete_file))
        .route("/url/{*path}", get(get_file_url))
        .with_state(app)
}

/// 查询参数：文件路径
#[derive(Debug, serde::Deserialize)]
struct FilePathQuery {
    path: String,
}

/// 上传文件（Multipart）
async fn upload_file(
    State(app): State<Arc<App>>,
    mut multipart: Multipart,
) -> Json<ApiResponse<crate::application::file::FileUploadResult>> {
    let service = app.inject::<FileService>();

    // 解析 multipart 中的第一个文件
    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field
            .file_name()
            .unwrap_or("unknown.bin")
            .to_string();
        let content_type = field
            .content_type()
            .map(|c| c.to_string());

        match field.bytes().await {
            Ok(data) => match service.upload(data.to_vec(), &file_name, content_type.as_deref()).await {
                Ok(result) => return Json(ApiResponse::success(result)),
                Err(e) => return Json(ApiResponse::error(400, e.to_string())),
            },
            Err(e) => return Json(ApiResponse::error(400, e.to_string())),
        }
    }

    Json(ApiResponse::error(400, "未上传文件"))
}

/// 下载文件
async fn download_file(
    State(app): State<Arc<App>>,
    Path(path): Path<String>,
) -> Result<(axum::http::StatusCode, [(String, String); 1], Vec<u8>), (axum::http::StatusCode, String)> {
    let service = app.inject::<FileService>();

    match service.download(&path).await {
        Ok(data) => {
            let content_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            Ok((
                axum::http::StatusCode::OK,
                [("Content-Type".to_string(), content_type)],
                data,
            ))
        }
        Err(e) => Err((axum::http::StatusCode::NOT_FOUND, e.to_string())),
    }
}

/// 删除文件
async fn delete_file(
    State(app): State<Arc<App>>,
    Query(query): Query<FilePathQuery>,
) -> Json<ApiResponse<()>> {
    let service = app.inject::<FileService>();

    match service.delete(&query.path).await {
        Ok(()) => Json(ApiResponse::ok()),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}

/// 获取文件访问 URL
async fn get_file_url(
    State(app): State<Arc<App>>,
    Path(path): Path<String>,
) -> Json<ApiResponse<String>> {
    let service = app.inject::<FileService>();

    match service.get_url(&path, 3600).await {
        Ok(url) => Json(ApiResponse::success(url)),
        Err(e) => Json(ApiResponse::error(400, e.to_string())),
    }
}
