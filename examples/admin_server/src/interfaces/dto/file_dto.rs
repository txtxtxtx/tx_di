//! 文件 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct FileDto {
    pub id: u64,
    pub config_id: Option<u64>,
    pub name: Option<String>,
    pub file_path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: i32,
    pub creator: Option<String>,
    pub created_at: String,
    pub updater: Option<String>,
    pub updated_at: String,
}

impl From<&crate::domain::file::File> for FileDto {
    fn from(f: &crate::domain::file::File) -> Self {
        Self {
            id: f.id,
            config_id: f.config_id,
            name: f.name.clone(),
            file_path: f.file_path.clone(),
            url: f.url.clone(),
            file_type: f.file_type.clone(),
            size: f.size,
            creator: f.creator.clone(),
            created_at: f.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updater: f.updater.clone(),
            updated_at: f.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateFileRequest {
    pub config_id: Option<u64>,
    pub name: Option<String>,
    pub file_path: String,
    pub url: String,
    pub file_type: Option<String>,
    pub size: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFileRequest {
    pub config_id: Option<u64>,
    pub name: Option<String>,
    pub file_path: Option<String>,
    pub url: Option<String>,
    pub file_type: Option<String>,
    pub size: Option<i32>,
}
