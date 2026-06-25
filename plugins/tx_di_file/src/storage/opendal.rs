//! 基于 OpenDAL 的统一文件存储实现
//!
//! 通过 OpenDAL `Operator` 支持本地文件系统、S3 等多种后端，
//! 配置切换仅需修改 TOML 中的 `backend` 字段。

use super::{error::map_opendal_error, FileInfo, FileStorage, guess_mime_type};
use crate::config::{FileConfig, StorageBackend};
use async_trait::async_trait;
use futures::Stream;
use opendal::Operator;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt};
use tx_error::{AppError, AppResult};

/// 基于 OpenDAL 的统一文件存储
///
/// # 支持的后端
/// - `local` — 本地文件系统 (opendal::services::Fs)
/// - `s3` — AWS S3 / MinIO (opendal::services::S3)
///
/// 未来可通过 feature flag 零成本扩展 GCS、Azure Blob 等。
#[derive(Debug, Clone)]
pub struct OpendalStorage {
    /// OpenDAL 操作器
    operator: Operator,
    /// 文件访问基础 URL（本地存储时用于拼接公开 URL）
    base_url: String,
}

impl OpendalStorage {
    /// 从 `FileConfig` 构建存储实例
    ///
    /// 根据 `config.backend` 自动选择后端并初始化 OpenDAL Operator。
    pub fn new(config: &FileConfig) -> AppResult<Self> {
        let operator = match config.backend {
            StorageBackend::Database => {
                return Err(anyhow::anyhow!("数据库存储后端不支持 OpendalStorage，请使用 create_storage()").into());
            }
            StorageBackend::Local => {
                let mut builder = opendal::services::Fs::default();
                builder = builder.root(&config.base_path);
                Operator::new(builder)
                    .map_err(|e| map_opendal_error(e, &config.base_path))?
                    .finish()
            }
            #[cfg(feature = "s3")]
            StorageBackend::S3 => {
                let mut builder = opendal::services::S3::default();
                builder = builder.bucket(&config.s3.bucket).region(&config.s3.region);

                if !config.s3.endpoint.is_empty() {
                    builder = builder.endpoint(&config.s3.endpoint);
                }

                // 默认 path-style（MinIO 兼容），显式启用 virtual-host style 需设为 false
                if !config.s3.force_path_style {
                    builder = builder.enable_virtual_host_style();
                }

                // 凭证配置
                if !config.s3.access_key.is_empty() && !config.s3.secret_key.is_empty() {
                    builder = builder
                        .access_key_id(&config.s3.access_key)
                        .secret_access_key(&config.s3.secret_key);
                }

                Operator::new(builder)
                    .map_err(|e| map_opendal_error(e, ""))?
                    .finish()
            }
            #[cfg(not(feature = "s3"))]
            StorageBackend::S3 => {
                return Err(AppError::from_anyhow(anyhow::anyhow!(
                    "S3 存储后端需要启用 's3' feature flag"
                )));
            }
        };

        tracing::info!(
            backend = ?config.backend,
            base_path = %config.base_path,
            "OpenDAL 存储后端已初始化"
        );

        Ok(Self {
            operator,
            base_url: config.base_url.clone(),
        })
    }

    /// 获取文件的公开访问 URL（仅本地存储）
    fn file_url(&self, path: &str) -> String {
        let safe_path = path.trim_start_matches('/');
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            safe_path
        )
    }
}

#[async_trait]
impl FileStorage for OpendalStorage {
    async fn write_stream(
        &self,
        path: &str,
        reader: &mut (dyn AsyncRead + Unpin + Send),
        _content_type: Option<&str>,
    ) -> AppResult<String> {
        // 确保父目录存在（本地存储需要）
        if let Some(parent) = std::path::Path::new(path).parent() {
            if let Some(parent_str) = parent.to_str() {
                if !parent_str.is_empty() {
                    // OpenDAL Fs 服务要求 create_dir 路径以 `/` 结尾
                    let dir_path = if parent_str.ends_with('/') {
                        parent_str.to_string()
                    } else {
                        format!("{}/", parent_str)
                    };
                    self.operator
                        .create_dir(&dir_path)
                        .await
                        .map_err(|e| map_opendal_error(e, path))?;
                }
            }
        }

        // 流式写入：通过 OpenDAL Writer 分块传输，不缓冲全文件
        let mut writer = self
            .operator
            .writer(path)
            .await
            .map_err(|e| map_opendal_error(e, path))?;

        let mut buf = vec![0u8; 8192];
        loop {
            let n = reader
                .read(&mut buf)
                .await
                .map_err(AppError::from)?;
            if n == 0 {
                break;
            }
            writer
                .write(buf[..n].to_vec())
                .await
                .map_err(|e| map_opendal_error(e, path))?;
        }

        writer
            .close()
            .await
            .map_err(|e| map_opendal_error(e, path))?;

        let path_str = path.to_string();
        tracing::debug!(path = %path_str, "文件已写入存储");
        Ok(path_str)
    }

    async fn read_stream(
        &self,
        path: &str,
    ) -> AppResult<Pin<Box<dyn AsyncRead + Send + Unpin>>> {
        use tokio_util::io::StreamReader;
        use futures::StreamExt;

        let reader = self
            .operator
            .reader(path)
            .await
            .map_err(|e| map_opendal_error(e, path))?;

        let bytes_stream = reader
            .into_bytes_stream(0..)
            .await
            .map_err(|e| map_opendal_error(e, path))?;

        let io_stream = bytes_stream.map(|r| {
            r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        });

        Ok(Box::pin(StreamReader::new(io_stream)))
    }

    async fn delete(&self, path: &str) -> AppResult<()> {
        self.operator
            .delete(path)
            .await
            .map_err(|e| map_opendal_error(e, path))?;
        tracing::debug!(path = %path, "文件已删除");
        Ok(())
    }

    async fn exists(&self, path: &str) -> AppResult<bool> {
        match self.operator.exists(path).await {
            Ok(exists) => Ok(exists),
            Err(e) => Err(map_opendal_error(e, path)),
        }
    }

    async fn info(&self, path: &str) -> AppResult<FileInfo> {
        let metadata = self
            .operator
            .stat(path)
            .await
            .map_err(|e| map_opendal_error(e, path))?;

        Ok(FileInfo {
            path: path.to_string(),
            size: metadata.content_length(),
            content_type: metadata
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| guess_mime_type(path)),
            url: Some(self.file_url(path)),
        })
    }

    async fn list_stream(
        &self,
        prefix: &str,
    ) -> AppResult<Pin<Box<dyn Stream<Item = AppResult<FileInfo>> + Send>>> {
        let prefix_owned = prefix.to_string();
        let operator = self.operator.clone();
        let base_url = self.base_url.clone();

        // 使用 async-stream 方式：先获取所有条目再转为 Stream
        // OpenDAL 的 list 返回 entries，分页由内部处理
        let entries = operator
            .list(&prefix_owned)
            .await
            .map_err(|e| map_opendal_error(e, &prefix_owned))?;

        let items: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                // 只返回文件，过滤目录
                e.metadata().mode().is_file()
            })
            .map(|entry| {
                let path = entry.path().to_string();
                let metadata = entry.metadata();
                let safe_path = path.trim_start_matches('/');
                let url = format!(
                    "{}/{}",
                    base_url.trim_end_matches('/'),
                    safe_path
                );
                Ok(FileInfo {
                    path,
                    size: metadata.content_length(),
                    content_type: metadata
                        .content_type()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| guess_mime_type(entry.name())),
                    url: Some(url),
                })
            })
            .collect();

        Ok(Box::pin(futures::stream::iter(items)))
    }

    async fn presigned_url(
        &self,
        path: &str,
        expire_secs: u64,
    ) -> AppResult<String> {
        // OpenDAL 的 presign 支持大部分后端（S3/GCS/Azure 等）
        // 本地文件系统则回退到 file_url
        match self
            .operator
            .presign_read(path, std::time::Duration::from_secs(expire_secs))
            .await
        {
            Ok(req) => Ok(req.uri().to_string()),
            Err(e) => {
                // 本地存储不支持签名，返回公开 URL
                tracing::debug!(
                    path = %path,
                    error = %e,
                    "后端不支持 presign，回退到公开 URL"
                );
                Ok(self.file_url(path))
            }
        }
    }
}
