//! 文件存储统一抽象
//!
//! 定义 `FileStorage` trait 及基于 OpenDAL 的统一后端实现。
//! 支持本地文件系统、S3 及任何 OpenDAL 支持的后端。

mod error;
mod opendal;

pub use error::{FileStorageErr, map_opendal_error};
pub use opendal::OpendalStorage;

use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt};
use tx_error::AppResult;

// ============================================================================
// 数据模型
// ============================================================================

/// 文件信息
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 文件存储路径（相对路径/Key）
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// MIME 类型
    pub content_type: String,
    /// 外部访问 URL（由存储后端提供）
    pub url: Option<String>,
}

// ============================================================================
// FileStorage trait（核心抽象）
// ============================================================================

/// 统一文件存储 trait
///
/// 所有存储后端必须实现此 trait。核心方法为流式读写，
/// 便捷方法 `upload` / `download` / `list` 提供默认实现，
/// 底层委托给流式方法。
///
/// # 示例
///
/// ```rust,ignore
/// use tx_di_file::storage::FileStorage;
///
/// async fn save_avatar(storage: &dyn FileStorage, user_id: &str, data: &[u8]) {
///     storage.upload(&format!("avatars/{}.png", user_id), data, Some("image/png"))
///         .await.unwrap();
/// }
/// ```
#[async_trait]
pub trait FileStorage: Send + Sync + std::fmt::Debug {
    // ========================================================================
    // 核心流式方法（必须实现）
    // ========================================================================

    /// 流式写入文件
    ///
    /// 接收 `AsyncRead` 流，将数据逐块写入存储后端，
    /// 避免将整个文件加载到内存。
    ///
    /// # 参数
    /// - `path`: 存储路径
    /// - `reader`: 可读取的数据流
    /// - `content_type`: MIME 类型（可选）
    async fn write_stream(
        &self,
        path: &str,
        reader: &mut (dyn AsyncRead + Unpin + Send),
        content_type: Option<&str>,
    ) -> AppResult<String>;

    /// 流式读取文件
    ///
    /// 返回实现 `AsyncRead + AsyncSeek` 的读取器，
    /// 调用方可边读边处理（如流式 HTTP 响应）。
    async fn read_stream(
        &self,
        path: &str,
    ) -> AppResult<Pin<Box<dyn AsyncRead + Send + Unpin>>>;

    /// 删除文件
    async fn delete(&self, path: &str) -> AppResult<()>;

    /// 检查文件是否存在
    async fn exists(&self, path: &str) -> AppResult<bool>;

    /// 获取文件信息
    async fn info(&self, path: &str) -> AppResult<FileInfo>;

    /// 流式列出目录下的文件
    ///
    /// 返回 `Stream`，支持增量消费，大数据量场景不爆内存。
    async fn list_stream(
        &self,
        prefix: &str,
    ) -> AppResult<Pin<Box<dyn Stream<Item = AppResult<FileInfo>> + Send>>>;

    /// 获取文件外部访问 URL（签名 URL 或公开 URL）
    async fn presigned_url(&self, path: &str, expire_secs: u64) -> AppResult<String>;

    // ========================================================================
    // 便捷方法（基于流式方法提供默认实现）
    // ========================================================================

    /// 上传字节数据（小文件便捷方法）
    ///
    /// 底层走 `write_stream`。大文件请直接使用 `write_stream`。
    async fn upload(
        &self,
        path: &str,
        data: &[u8],
        content_type: Option<&str>,
    ) -> AppResult<String> {
        let mut cursor = std::io::Cursor::new(data);
        self.write_stream(path, &mut cursor, content_type).await
    }

    /// 下载文件为字节数组（小文件便捷方法）
    ///
    /// 底层走 `read_stream`。大文件请直接使用 `read_stream`。
    async fn download(&self, path: &str) -> AppResult<Vec<u8>> {
        let mut reader = self.read_stream(path).await?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    /// 列出目录下所有文件（便捷方法）
    ///
    /// 底层走 `list_stream` 并收集为 `Vec`。
    async fn list(&self, prefix: &str) -> AppResult<Vec<FileInfo>> {
        use futures::StreamExt;
        let stream = self.list_stream(prefix).await?;
        // 使用 pin_mut 避免额外的 Box 包装
        futures::pin_mut!(stream);
        let mut files = Vec::new();
        while let Some(item) = stream.next().await {
            files.push(item?);
        }
        Ok(files)
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 根据扩展名推断 MIME 类型
pub fn guess_mime_type(path: &str) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string()
}

/// 从路径提取扩展名（小写）
pub fn extract_extension(path: &str) -> Option<String> {
    PathBuf::from(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}
