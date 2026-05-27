//! 本地文件系统存储实现

use super::{FileInfo, FileStorage, FileStorageError, UploadParams, guess_mime_type};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 本地文件系统存储
///
/// 将文件存储在服务器本地磁盘上。
///
/// # 示例
///
/// ```rust,ignore
/// let storage = LocalFileStorage::new("./uploads", "http://localhost:8080/files");
/// storage.upload(&UploadParams { data: b"hello", path: "test.txt", content_type: None }).await?;
/// ```
#[derive(Debug, Clone)]
pub struct LocalFileStorage {
    /// 文件存储根目录
    base_path: PathBuf,
    /// 文件访问基础 URL
    base_url: String,
}

impl LocalFileStorage {
    /// 创建本地文件存储
    ///
    /// # 参数
    /// - `base_path`: 文件存储根目录
    /// - `base_url`: 文件访问基础 URL（用于生成 presigned_url）
    pub fn new(base_path: impl Into<PathBuf>, base_url: impl Into<String>) -> Self {
        let base_path = base_path.into();
        let base_url = base_url.into();

        // 确保目录存在
        std::fs::create_dir_all(&base_path).ok();

        Self {
            base_path,
            base_url,
        }
    }

    /// 获取文件的完整本地路径
    fn full_path(&self, path: &str) -> PathBuf {
        // 防止路径穿越
        let safe_path = path.trim_start_matches('/').trim_start_matches('\\');
        self.base_path.join(safe_path)
    }

    /// 获取文件的公开 URL
    fn file_url(&self, path: &str) -> String {
        let safe_path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url.trim_end_matches('/'), safe_path)
    }
}

#[async_trait]
impl FileStorage for LocalFileStorage {
    async fn upload(&self, params: UploadParams<'_>) -> Result<String, FileStorageError> {
        let full_path = self.full_path(params.path);

        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&full_path, params.data).await?;

        let path_str = params.path.to_string();
        tracing::debug!(path = %path_str, size = params.data.len(), "文件已上传到本地存储");
        Ok(path_str)
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>, FileStorageError> {
        let full_path = self.full_path(path);
        if !full_path.exists() {
            return Err(FileStorageError::NotFound(path.to_string()));
        }
        Ok(fs::read(&full_path).await?)
    }

    async fn delete(&self, path: &str) -> Result<(), FileStorageError> {
        let full_path = self.full_path(path);
        if !full_path.exists() {
            return Err(FileStorageError::NotFound(path.to_string()));
        }
        fs::remove_file(&full_path).await?;
        tracing::debug!(path = %path, "文件已从本地存储删除");
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, FileStorageError> {
        let full_path = self.full_path(path);
        Ok(full_path.exists())
    }

    async fn info(&self, path: &str) -> Result<FileInfo, FileStorageError> {
        let full_path = self.full_path(path);
        if !full_path.exists() {
            return Err(FileStorageError::NotFound(path.to_string()));
        }
        let metadata = fs::metadata(&full_path).await?;
        Ok(FileInfo {
            path: path.to_string(),
            size: metadata.len(),
            content_type: guess_mime_type(path),
            url: Some(self.file_url(path)),
        })
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileInfo>, FileStorageError> {
        let dir_path = self.full_path(prefix);
        if !dir_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&dir_path).await?;
        let mut files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let relative = path
                    .strip_prefix(&self.base_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let metadata = entry.metadata().await?;
                files.push(FileInfo {
                    path: relative,
                    size: metadata.len(),
                    content_type: guess_mime_type(&entry.file_name().to_string_lossy()),
                    url: Some(self.file_url(&entry.file_name().to_string_lossy())),
                });
            }
        }
        Ok(files)
    }

    async fn presigned_url(&self, path: &str, _expire_secs: u64) -> Result<String, FileStorageError> {
        // 本地存储不支持签名 URL，直接返回公开 URL
        Ok(self.file_url(path))
    }
}
