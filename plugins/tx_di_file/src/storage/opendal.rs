//! 基于 OpenDAL 的统一文件存储实现
//!
//! 通过 OpenDAL `Operator` 支持本地文件系统、S3 等多种后端，
//! 配置切换仅需修改 TOML 中的 `backend` 字段。

use super::{FileInfo, FileStorage, guess_mime_type};
use crate::config::{FileConfig, S3Config, StorageBackend, StorageConfig};
use crate::error::{map_opendal_error, FilePluginErr};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
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
    /// 存储后端类型（用于后端特定行为，如本地需 create_dir）
    backend: StorageBackend,
}

impl OpendalStorage {
    /// 从 `FileConfig` 构建本地存储实例
    ///
    /// 仅提取 `base_path` 和 `base_url` 创建 `sys:local` 对应的后端。
    /// 如需创建 S3 或其他后端，请使用 `from_storage_config` / `new_s3`。
    #[deprecated(since = "0.2.0", note = "请使用 `new_local()` 代替")]
    pub fn new(config: &FileConfig) -> AppResult<Self> {
        Self::new_local(&config.base_path, &config.base_url)
    }

    /// 从 `StorageConfig` 构建存储实例
    ///
    /// 根据 `cfg.backend` 自动选择后端并初始化 OpenDAL Operator。
    pub fn from_storage_config(cfg: &StorageConfig) -> AppResult<Self> {
        let operator = match cfg.backend {
            StorageBackend::Database => {
                return Err(AppError::with_context(
                    FilePluginErr::StorageInitFailed,
                    "数据库存储后端不支持 OpendalStorage",
                ));
            }
            StorageBackend::Local => {
                let mut builder = opendal::services::Fs::default();
                builder = builder.root(&cfg.base_path);
                Operator::new(builder)
                    .map_err(|e| map_opendal_error(e, &cfg.base_path))?
                    .finish()
            }
            #[cfg(feature = "s3")]
            StorageBackend::S3 => Self::build_s3_operator(&cfg.s3)?
                .ok_or_else(|| AppError::with_context(
                    FilePluginErr::StorageInitFailed,
                    "S3 配置不完整",
                ))?,
            #[cfg(not(feature = "s3"))]
            StorageBackend::S3 => {
                return Err(AppError::with_context(
                    FilePluginErr::StorageInitFailed,
                    "S3 存储后端需要启用 's3' feature flag",
                ));
            }
        };

        tracing::info!(
            backend = ?cfg.backend,
            name = %cfg.name,
            "OpenDAL 存储后端已初始化"
        );

        Ok(Self {
            operator,
            base_url: cfg.base_url.clone(),
            backend: cfg.backend.clone(),
        })
    }

    /// 创建本地文件系统存储
    pub fn new_local(base_path: &str, base_url: &str) -> AppResult<Self> {
        let cfg = StorageConfig {
            name: String::new(),
            backend: StorageBackend::Local,
            base_path: base_path.to_string(),
            base_url: base_url.to_string(),
            s3: S3Config::default(),
        };
        Self::from_storage_config(&cfg)
    }

    /// 创建 S3 存储
    pub fn new_s3(s3_cfg: &S3Config, base_url: &str) -> AppResult<Self> {
        let cfg = StorageConfig {
            name: String::new(),
            backend: StorageBackend::S3,
            base_path: String::new(),
            base_url: base_url.to_string(),
            s3: s3_cfg.clone(),
        };
        Self::from_storage_config(&cfg)
    }

    /// 从 `FileConfig`（旧格式）构建 S3 Operator，兼容旧配置中的 `s3` 字段
    fn build_s3_operator(s3: &S3Config) -> AppResult<Option<Operator>> {
        if s3.bucket.is_empty() {
            return Ok(None);
        }
        let mut builder = opendal::services::S3::default();
        builder = builder.bucket(&s3.bucket).region(&s3.region);

        if !s3.endpoint.is_empty() {
            builder = builder.endpoint(&s3.endpoint);
        }

        // 默认 path-style（MinIO 兼容），显式启用 virtual-host style 需设为 false
        if !s3.force_path_style {
            builder = builder.enable_virtual_host_style();
        }

        // 凭证配置
        if !s3.access_key.is_empty() && !s3.secret_key.is_empty() {
            builder = builder
                .access_key_id(&s3.access_key)
                .secret_access_key(&s3.secret_key);
        }

        let op = Operator::new(builder)
            .map_err(|e| map_opendal_error(e, ""))?
            .finish();
        Ok(Some(op))
    }

    /// 获取文件的公开访问 URL
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
        content_type: Option<&str>,
    ) -> AppResult<String> {
        // 本地文件系统需先确保父目录存在
        if self.backend == StorageBackend::Local {
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
        }

        // 流式写入：通过 OpenDAL Writer 分块传输，不缓冲全文件
        // 若有 content_type 则通过 writer_with 传入
        let mut writer = {
            let builder = self.operator.writer_with(path);
            if let Some(ct) = content_type {
                builder.content_type(ct).await
            } else {
                builder.await
            }
        }
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
        // 使用 lister 实现真·流式列出，避免一次性加载全部条目
        let lister = self
            .operator
            .lister(prefix)
            .await
            .map_err(|e| map_opendal_error(e, prefix))?;

        let base_url_trimmed = self.base_url.trim_end_matches('/').to_string();

        let stream = lister.filter_map(move |entry| {
            let base_url = base_url_trimmed.clone();
            async move {
                match entry {
                    Ok(e) if e.metadata().mode().is_file() => {
                        let path = e.path().to_string();
                        let metadata = e.metadata();
                        let safe_path = path.trim_start_matches('/');
                        let url = format!("{}/{}", base_url, safe_path);
                        Some(Ok(FileInfo {
                            path,
                            size: metadata.content_length(),
                            content_type: metadata
                                .content_type()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| guess_mime_type(e.name())),
                            url: Some(url),
                        }))
                    }
                    Ok(_) => None, // 跳过目录
                    Err(err) => Some(Err(map_opendal_error(err, ""))),
                }
            }
        });

        Ok(Box::pin(stream))
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
            Err(e) if e.kind() == opendal::ErrorKind::Unsupported => {
                // 本地存储不支持签名，回退到公开 URL
                tracing::debug!(
                    path = %path,
                    "后端不支持 presign，回退到公开 URL"
                );
                Ok(self.file_url(path))
            }
            Err(e) => {
                // 真实错误（认证失败、网络超时等）不应静默回退
                Err(map_opendal_error(e, path))
            }
        }
    }
}
