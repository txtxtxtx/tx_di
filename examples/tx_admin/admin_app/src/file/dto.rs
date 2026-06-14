use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileCommand {
    pub name: String,
    pub path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: i32,
    pub config_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileQueryRequest {
    pub name: Option<String>,
    pub file_type: Option<String>,
    pub config_id: Option<i32>,
    pub page: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FileResponse {
    pub id: u64,
    pub config_id: Option<i32>,
    pub name: String,
    pub path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: i32,
}

impl From<admin_domain::file::model::aggregate::File> for FileResponse {
    fn from(file: admin_domain::file::model::aggregate::File) -> Self {
        Self {
            id: file.id,
            config_id: file.config_id,
            name: file.name,
            path: file.path,
            url: file.url,
            file_type: file.file_type,
            size: file.size,
        }
    }
}

/// 文件下载响应
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FileDownloadResponse {
    pub url: String,
    pub filename: String,
    pub size: i32,
    pub content_type: String,
}

impl From<admin_domain::file::model::value_object::FileDownloadInfo> for FileDownloadResponse {
    fn from(info: admin_domain::file::model::value_object::FileDownloadInfo) -> Self {
        Self {
            url: info.url,
            filename: info.filename,
            size: info.size,
            content_type: info.content_type,
        }
    }
}
