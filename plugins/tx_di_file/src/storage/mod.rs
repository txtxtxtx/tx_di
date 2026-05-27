//! 文件存储统一抽象
//!
//! 定义 `FileStorage` trait 及不同后端的实现。

mod local;
#[cfg(feature = "s3")]
mod s3_pkg;

use std::fmt::Debug;
pub use local::LocalFileStorage;
#[cfg(feature = "s3")]
pub use s3_pkg::S3FileStorage;

use async_trait::async_trait;
use std::path::PathBuf;

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

/// 文件上传参数
#[derive(Debug, Clone)]
pub struct UploadParams<'a> {
    /// 文件数据
    pub data: &'a [u8],
    /// 存储路径（相对路径/Key）
    pub path: &'a str,
    /// MIME 类型（可选，默认从扩展名推断）
    pub content_type: Option<&'a str>,
}

/// 统一文件存储 trait
///
/// 所有存储后端必须实现此 trait，上层业务代码无需关心底层是
/// 本地文件系统还是对象存储服务。
///
/// # 示例
///
/// ```rust,ignore
/// use tx_di_file::storage::FileStorage;
///
/// async fn save_avatar(storage: &dyn FileStorage, user_id: &str, data: &[u8]) {
///     storage.upload(&UploadParams {
///         data,
///         path: &format!("avatars/{}.png", user_id),
///         content_type: Some("image/png"),
///     }).await.unwrap();
/// }
/// ```
#[async_trait]
pub trait FileStorage: Send + Sync + Debug {
    /// 上传文件
    ///
    /// 如果路径已存在则覆盖。
    /// 返回文件的存储路径。
    async fn upload(&self, params: UploadParams<'_>) -> Result<String, FileStorageError>;

    /// 下载文件
    ///
    /// 返回文件二进制数据。
    async fn download(&self, path: &str) -> Result<Vec<u8>, FileStorageError>;

    /// 删除文件
    async fn delete(&self, path: &str) -> Result<(), FileStorageError>;

    /// 检查文件是否存在
    async fn exists(&self, path: &str) -> Result<bool, FileStorageError>;

    /// 获取文件信息
    async fn info(&self, path: &str) -> Result<FileInfo, FileStorageError>;

    /// 列出目录下的文件
    async fn list(&self, prefix: &str) -> Result<Vec<FileInfo>, FileStorageError>;

    /// 获取文件外部访问 URL（有有效期限制的签名 URL 或公开 URL）
    async fn presigned_url(&self, path: &str, expire_secs: u64) -> Result<String, FileStorageError>;
}

/// 文件存储错误类型
#[derive(Debug, thiserror::Error)]
pub enum FileStorageError {
    /// 文件未找到
    #[error("文件未找到: {0}")]
    NotFound(String),

    /// 文件已存在且不允许覆盖
    #[error("文件已存在: {0}")]
    AlreadyExists(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// S3 / 网络错误
    #[error("存储服务错误: {0}")]
    Storage(String),

    /// 不支持的操作
    #[error("不支持的操作: {0}")]
    Unsupported(String),

    /// 文件大小超限
    #[error("文件大小超限: {size} > {limit}")]
    FileTooLarge { size: u64, limit: u64 },

    /// 扩展名不允许
    #[error("不允许的文件类型: {ext}")]
    InvalidExtension { ext: String },
}

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
