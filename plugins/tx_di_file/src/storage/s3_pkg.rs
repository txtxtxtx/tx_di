//! S3 对象存储实现
//!
//! 支持 AWS S3 及兼容 S3 协议的对象存储（如 MinIO）。

use super::{FileInfo, FileStorage, FileStorageError, UploadParams, guess_mime_type};
use async_trait::async_trait;
use aws_sdk_s3::{
    config::{BehaviorVersion, Region},
    primitives::ByteStream,
    Client,
};
use bytes::Bytes;

/// S3 对象存储
///
/// 支持 AWS S3 及兼容 S3 协议的对象存储（如 MinIO）。
///
/// # 示例
///
/// ```rust,ignore
/// let storage = S3FileStorage::new(
///     "my-bucket",
///     "ap-southeast-1",
///     Some("http://localhost:9000"),  // MinIO 端点
///     Some("access_key"),
///     Some("secret_key"),
///     true,  // force_path_style for MinIO
/// ).await?;
/// ```
#[derive(Debug, Clone)]
pub struct S3FileStorage {
    client: Client,
    bucket: String,
}

impl S3FileStorage {
    /// 创建 S3 存储实例
    pub async fn new(
        bucket: impl Into<String>,
        region: impl Into<String>,
        endpoint: Option<&str>,
        access_key: Option<&str>,
        secret_key: Option<&str>,
        force_path_style: bool,
    ) -> Result<Self, FileStorageError> {
        let bucket = bucket.into();
        let region = Region::new(region.into());

        let mut config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(region);

        if let Some(endpoint) = endpoint {
            if !endpoint.is_empty() {
                config_builder = config_builder.endpoint_url(endpoint);
            }
        }

        // 使用凭证（如果提供了 access_key 和 secret_key）
        let config = if let (Some(ak), Some(sk)) = (access_key, secret_key) {
            if !ak.is_empty() && !sk.is_empty() {
                let creds = aws_credential_types::Credentials::new(
                    ak, sk, None, None, "tx-di-file",
                );
                config_builder
                    .credentials_provider(creds)
                    .load()
                    .await
            } else {
                config_builder.load().await
            }
        } else {
            config_builder.load().await
        };

        let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
        if force_path_style {
            s3_config = s3_config.force_path_style(true);
        }

        let client = Client::from_conf(s3_config.build());

        // 验证连接：尝试列出 bucket
        client
            .head_bucket()
            .bucket(&bucket)
            .send()
            .await
            .map_err(|e| FileStorageError::Storage(format!("S3 连接失败: {}", e)))?;

        tracing::info!(bucket = %bucket, "S3 存储已连接");
        Ok(Self { client, bucket })
    }
}

#[async_trait]
impl FileStorage for S3FileStorage {
    async fn upload(&self, params: UploadParams<'_>) -> Result<String, FileStorageError> {
        let content_type = params
            .content_type
            .map(|s| s.to_string())
            .unwrap_or_else(|| guess_mime_type(params.path));

        let body = ByteStream::from(Bytes::copy_from_slice(params.data));

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(params.path)
            .body(body)
            .content_type(&content_type)
            .send()
            .await
            .map_err(|e| FileStorageError::Storage(format!("S3 上传失败: {}", e)))?;

        let path_str = params.path.to_string();
        tracing::debug!(path = %path_str, size = params.data.len(), "文件已上传到 S3");
        Ok(path_str)
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>, FileStorageError> {
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    FileStorageError::NotFound(path.to_string())
                } else {
                    FileStorageError::Storage(format!("S3 下载失败: {}", e))
                }
            })?;

        let data = result
            .body
            .collect()
            .await
            .map_err(|e| FileStorageError::Storage(format!("S3 读取数据失败: {}", e)))?;

        Ok(data.to_vec())
    }

    async fn delete(&self, path: &str) -> Result<(), FileStorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| FileStorageError::Storage(format!("S3 删除失败: {}", e)))?;

        tracing::debug!(path = %path, "文件已从 S3 删除");
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, FileStorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(FileStorageError::Storage(format!("S3 检查失败: {}", e)))
                }
            }
        }
    }

    async fn info(&self, path: &str) -> Result<FileInfo, FileStorageError> {
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    FileStorageError::NotFound(path.to_string())
                } else {
                    FileStorageError::Storage(format!("S3 获取信息失败: {}", e))
                }
            })?;

        Ok(FileInfo {
            path: path.to_string(),
            size: result.content_length().unwrap_or(0) as u64,
            content_type: result
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string(),
            url: None,
        })
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileInfo>, FileStorageError> {
        let mut files = Vec::new();
        let mut continuation_token = None;

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);

            if let Some(token) = continuation_token {
                req = req.continuation_token(token);
            }

            let result = req
                .send()
                .await
                .map_err(|e| FileStorageError::Storage(format!("S3 列表失败: {}", e)))?;

            if let Some(contents) = result.contents() {
                for obj in contents {
                    if let Some(key) = obj.key() {
                        files.push(FileInfo {
                            path: key.to_string(),
                            size: obj.size().unwrap_or(0) as u64,
                            content_type: guess_mime_type(key),
                            url: None,
                        });
                    }
                }
            }

            if result.is_truncated().unwrap_or(false) {
                continuation_token = result.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(files)
    }

    async fn presigned_url(&self, path: &str, expire_secs: u64) -> Result<String, FileStorageError> {
        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(path);

        let presigned = req
            .presigned(
                aws_sdk_s3::presigning::PresigningConfig::expires_in(
                    std::time::Duration::from_secs(expire_secs),
                )
                .map_err(|e| FileStorageError::Storage(format!("生成签名 URL 失败: {}", e)))?,
            )
            .await
            .map_err(|e| FileStorageError::Storage(format!("生成签名 URL 失败: {}", e)))?;

        Ok(presigned.uri().to_string())
    }
}
