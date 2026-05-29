//! 文件应用服务
//!
//! 封装文件上传/下载/删除等业务逻辑，调用 `FileStorage` trait 完成存储操作。

use std::sync::Arc;
use tx_di_core::tx_comp;
use tx_di_file::storage::{FileStorage, UploadParams};
use tx_di_file::FilePlugin;

/// 文件上传结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileUploadResult {
    pub path: String,
    pub url: String,
    pub size: u64,
    pub content_type: String,
}

/// 文件应用服务
#[derive(Debug)]
#[tx_comp]
pub struct FileService {
    pub file_plugin: Arc<FilePlugin>,
}

impl FileService {
    /// 上传文件
    pub async fn upload(
        &self,
        data: Vec<u8>,
        original_name: &str,
        content_type: Option<&str>,
    ) -> Result<FileUploadResult, anyhow::Error> {
        let storage = self.file_plugin.storage();
        let path = Self::generate_path(original_name);

        let params = UploadParams {
            data: &data,
            path: &path,
            content_type,
        };

        let stored_path = storage.upload(params).await?;
        let url = storage.presigned_url(&stored_path, 3600).await?;

        Ok(FileUploadResult {
            path: stored_path,
            url,
            size: data.len() as u64,
            content_type: content_type
                .unwrap_or("application/octet-stream")
                .to_string(),
        })
    }

    /// 下载文件
    pub async fn download(&self, path: &str) -> Result<Vec<u8>, anyhow::Error> {
        let storage = self.file_plugin.storage();
        storage.download(path).await.map_err(Into::into)
    }

    /// 删除文件
    pub async fn delete(&self, path: &str) -> Result<(), anyhow::Error> {
        let storage = self.file_plugin.storage();
        storage.delete(path).await.map_err(Into::into)
    }

    /// 获取文件访问 URL
    pub async fn get_url(&self, path: &str, expire_secs: u64) -> Result<String, anyhow::Error> {
        let storage = self.file_plugin.storage();
        storage.presigned_url(path, expire_secs).await.map_err(Into::into)
    }

    /// 生成存储路径
    /// 格式：{yyyy-MM}/{uuid}.{ext}
    fn generate_path(original_name: &str) -> String {
        let now = chrono::Utc::now();
        let date_prefix = now.format("%Y-%m").to_string();
        let ext = std::path::Path::new(original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let uuid = uuid::Uuid::new_v4();

        format!("{}/{}.{}", date_prefix, uuid, ext)
    }
}
