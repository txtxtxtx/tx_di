use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileQuery {
    pub name: Option<String>,
    pub file_type: Option<String>,
    pub config_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadCommand {
    pub name: String,
    pub path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: i32,
    pub config_id: Option<i32>,
}

/// 文件下载信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDownloadInfo {
    pub url: String,
    pub filename: String,
    pub size: i32,
    pub content_type: String,
}
