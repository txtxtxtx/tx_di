use admin_proto::{FileResponse, DownloadFileResponse};
use serde::{Deserialize, Serialize};

// ── Command / Query DTOs ──────────────────────────────────────────────

/// 上传文件命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileCommand {
    pub name: String,
    pub path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: i32,
    pub config_id: Option<i32>,
}

/// 文件分页查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileQueryRequest {
    pub name: Option<String>,
    pub file_type: Option<String>,
    pub config_id: Option<i32>,
    pub page: i64,
    pub size: i64,
}

// ── Conversion ────────────────────────────────────────────────────────

/// 领域模型 → Proto 响应：文件
pub fn file_to_response(file: admin_domain::file::model::aggregate::File) -> FileResponse {
    FileResponse {
        id: file.id,
        config_id: file.config_id,
        name: file.name,
        path: file.path,
        url: file.url,
        file_type: file.file_type,
        size: file.size,
    }
}

/// 领域模型 → Proto 响应：文件下载
pub fn file_download_to_response(info: admin_domain::file::model::value_object::FileDownloadInfo) -> DownloadFileResponse {
    DownloadFileResponse {
        url: info.url,
        filename: info.filename,
        size: info.size as u64,
        content_type: info.content_type,
    }
}
