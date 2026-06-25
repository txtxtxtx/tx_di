use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

use admin_domain::file::model::aggregate::FileConfig;
use admin_domain::file::model::value_object::{FileQuery, FileUploadCommand};
use admin_domain::file::service::FileService;
use admin_domain::file::repository::FileConfigRepository;
use admin_domain::shared::model::Entity;
use admin_proto::{ListFilesRequest, FileResponse};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_file::{user_key, FilePluginErr, StorageConfig};
use tx_di_file::storage::{guess_mime_type, extract_extension, FileStorageErr, FileStorage, OpendalStorage};
use tx_di_file::FilePlugin;
use tx_error::{AppError, AppResult};

use crate::file::dto::{file_to_response, DownloadFileStream, PreviewUrlResponse};

/// 轻量字节计数器 —— 只计数不限制（大小限制由 axum DefaultBodyLimit 负责）
struct CountingReader<R> {
    inner: R,
    bytes_read: u64,
}

impl<R> CountingReader<R> {
    fn new(inner: R) -> Self {
        Self { inner, bytes_read: 0 }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for CountingReader<R> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let poll = Pin::new(&mut this.inner).poll_read(cx, buf);
        this.bytes_read += (buf.filled().len() - before) as u64;
        poll
    }
}

#[tx_comp]
pub struct FileAppService {
    pub file_service: Arc<FileService>,
    pub file_plugin: Arc<FilePlugin>,
    pub file_config_repo: Arc<dyn FileConfigRepository>,
}

impl FileAppService {
    pub fn new(
        file_service: Arc<FileService>,
        file_plugin: Arc<FilePlugin>,
        file_config_repo: Arc<dyn FileConfigRepository>,
    ) -> Self {
        Self {
            file_service,
            file_plugin,
            file_config_repo,
        }
    }

    // ========================================================================
    // 内部工具
    // ========================================================================

    /// 获取存储后端 —— 优先 DB 配置，回退到插件默认配置
    ///
    /// - `config_id` 为 Some 时查找指定配置；找不到则回退主配置
    /// - 为 None 时查找主配置
    /// - DB 无配置时回退到 FilePlugin 的 TOML 配置
    async fn get_storage(&self, config_id: Option<i32>) -> AppResult<Arc<dyn FileStorage>> {
        // 尝试从 DB 获取配置：指定 ID → 回退主配置（兼容已删除配置的旧文件）
        let db_config = if let Some(cid) = config_id {
            match self.file_config_repo.find_by_id(cid).await? {
                Some(cfg) => Some(cfg),
                None => self.file_config_repo.find_master().await?,
            }
        } else {
            self.file_config_repo.find_master().await?
        };

        if let Some(cfg) = db_config {
            if cfg.storage == 2 {
                return Err(AppError::with_context(
                    FileStorageErr::NotFound,
                    "数据库存储后端不适用此操作",
                ));
            }

            // 尝试从插件缓存中获取
            let key = user_key(&format!("db_{}", cfg.id()));
            if let Some(storage) = self.file_plugin.get_storage(&key) {
                return Ok(storage);
            }

            // 未缓存则从 DB 配置创建并注册到插件
            if let Ok(storage_cfg) = serde_json::from_str::<StorageConfig>(&cfg.config) {
                if let Ok(storage) = OpendalStorage::from_storage_config(&storage_cfg) {
                    let storage = Arc::new(storage) as Arc<dyn FileStorage>;
                    self.file_plugin.add_storage(key, storage.clone());
                    return Ok(storage);
                }
            }
        }

        // 回退到插件默认存储
        self.file_plugin
            .default_storage()
            .ok_or_else(|| FilePluginErr::DefaultStorageNotFound.into())
    }

    /// 获取本地文件服务的根目录
    pub async fn serve_base_dir(&self, config_id: Option<i32>) -> Option<String> {
        let db_config = if let Some(cid) = config_id {
            self.file_config_repo.find_by_id(cid).await.ok().flatten()
        } else {
            self.file_config_repo.find_master().await.ok().flatten()
        };
        if let Some(cfg) = db_config {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&cfg.config) {
                if let Some(base) = json.get("base_path").and_then(|v| v.as_str()) {
                    return Some(base.to_string());
                }
            }
        }
        // 回退到插件配置
        Some(self.file_plugin.config.base_path.clone())
    }

    // ========================================================================
    // 流式上传
    // ========================================================================

    pub async fn upload_file_stream(
        &self,
        filename: String,
        content_type: String,
        reader: &mut (dyn AsyncRead + Unpin + Send),
        config_id: Option<i32>,
        creator: Option<String>,
    ) -> AppResult<FileResponse> {
        // 1. 获取存储后端
        let storage = self.get_storage(config_id).await?;

        // 1.1 解析实际使用的 config_id（业务规则：未指定则用主配置）
        let resolved_config_id = self.file_service.resolve_config_id(config_id).await?;

        // 2. 校验扩展名 —— 领域层 DB 白名单 + 插件 TOML 回退
        let allowed = self.get_allowed_extensions(config_id).await;
        if !allowed.is_empty() {
            let ext = extract_extension(&filename).unwrap_or_default();
            if !allowed.contains(&ext) {
                return Err(AppError::with_context(
                    FileStorageErr::InvalidExtension,
                    format!("不允许的文件类型: .{}", ext),
                ));
            }
        }

        // 3. 字节计数
        let mut counting = CountingReader::new(reader);

        // 4. 生成存储路径: YYYY/MM/uuid_filename.ext
        let now = jiff::Zoned::now();
        let file_uuid = uuid::Uuid::now_v7();
        let path = format!(
            "{:04}/{:02}/{}_{}",
            now.year(),
            now.month(),
            file_uuid,
            filename
        );

        // 5. 流式写入存储后端
        let storage_path = storage
            .write_stream(&path, &mut counting, Some(&content_type))
            .await?;

        // 6. 获取实际写入大小
        let actual_size = counting.bytes_read as i32;

        // 7. 推断 MIME
        let mime = if content_type.is_empty() || content_type == "application/octet-stream" {
            guess_mime_type(&filename)
        } else {
            content_type
        };

        // 8. 生成访问 URL
        let url = storage
            .presigned_url(&storage_path, 3600)
            .await
            .unwrap_or_else(|_| storage_path.clone());

        // 9. 持久化元数据到 DB（config_id 始终为实际使用的配置 ID，不会为 NULL）
        let cmd = FileUploadCommand {
            name: filename,
            path: storage_path,
            url,
            file_type: Some(mime),
            size: actual_size,
            config_id: resolved_config_id,
        };
        let file = self.file_service.upload_file(cmd, creator).await?;
        Ok(file_to_response(file))
    }

    /// 获取允许的文件扩展名列表（领域 DB 白名单 + 插件 TOML 回退）
    async fn get_allowed_extensions(&self, config_id: Option<i32>) -> Vec<String> {
        // 先从领域层读取 DB 配置中的白名单
        let mut allowed = self
            .file_service
            .get_allowed_extensions(config_id)
            .await
            .unwrap_or_default();

        // DB 无配置时回退到插件 TOML 默认值
        if allowed.is_empty() {
            allowed = self.file_plugin.config.allowed_extensions.clone();
        }
        allowed
    }

    // ========================================================================
    // 流式下载
    // ========================================================================

    pub async fn download_file_stream(&self, file_id: u64) -> AppResult<DownloadFileStream> {
        let file = self.file_service.get_file(file_id).await?;
        let storage = self.get_storage(file.config_id).await?;
        let reader = storage.read_stream(&file.path).await?;

        let content_type = file
            .file_type
            .as_deref()
            .unwrap_or("application/octet-stream")
            .to_string();

        Ok(DownloadFileStream {
            reader,
            filename: file.name,
            content_type,
            size: file.size as u64,
        })
    }

    // ========================================================================
    // 预览 URL
    // ========================================================================

    /// 获取文件预览地址
    ///
    /// - 本地存储 (storage=0): 返回永久 URL → `/api/file/pre/serve/{path}`
    /// - S3 存储   (storage=1): 返回预签名临时 URL + 过期时间
    /// - 数据库     (storage=2): 暂存到本地 → 同本地
    pub async fn get_preview_url(&self, file_id: u64) -> AppResult<PreviewUrlResponse> {
        let file = self.file_service.get_file(file_id).await?;
        let storage = self.get_storage(file.config_id).await?;

        // 获取存储配置以判断类型
        // config_id 为空时回退到主配置，而非硬编码本地
        let storage_type = if let Some(cid) = file.config_id {
            self.file_config_repo
                .find_by_id(cid)
                .await?
                .map(|c| c.storage)
                .unwrap_or(0)
        } else {
            self.file_config_repo
                .find_master()
                .await?
                .map(|c| c.storage)
                .unwrap_or(0)
        };

        match storage_type {
            1 => {
                // S3: 生成预签名 URL（2 小时有效）
                let url = storage.presigned_url(&file.path, 7200).await?;
                let now = jiff::Timestamp::now();
                let expires_at = now
                    .saturating_add(jiff::SignedDuration::from_secs(7200))
                    .unwrap_or(now)
                    .to_string();
                Ok(PreviewUrlResponse {
                    url,
                    url_type: "temporary".into(),
                    expires_at: Some(expires_at),
                })
            }
            _ => {
                // 本地 / 数据库: 永久 URL，走 serve 路由（带 cid 定位正确的 base_path）
                let safe_path = file.path.trim_start_matches('/');
                let url = if let Some(cid) = file.config_id {
                    format!("/api/file/pre/serve/{}?cid={}", safe_path, cid)
                } else {
                    format!("/api/file/pre/serve/{}", safe_path)
                };
                Ok(PreviewUrlResponse {
                    url,
                    url_type: "permanent".into(),
                    expires_at: None,
                })
            }
        }
    }

    // ========================================================================
    // 删除
    // ========================================================================

    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        let file = self.file_service.get_file(file_id).await?;

        let storage = self.get_storage(file.config_id).await?;
        if let Err(e) = storage.delete(&file.path).await {
            tracing::warn!(
                file_id = file_id,
                path = %file.path,
                error = %e,
                "删除物理文件失败，继续执行 DB 软删除"
            );
        }

        self.file_service.delete_file(file_id, updater).await
    }

    // ========================================================================
    // 查询
    // ========================================================================

    pub async fn get_file_page(
        &self,
        req: ListFilesRequest,
    ) -> AppResult<Page<FileResponse>> {
        let query = FileQuery {
            name: req.name,
            file_type: req.file_type,
            config_id: req.config_id,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.file_service.get_file_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(file_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    pub async fn get_file(&self, file_id: u64) -> AppResult<FileResponse> {
        let file = self.file_service.get_file(file_id).await?;
        Ok(file_to_response(file))
    }

    // ========================================================================
    // 文件配置 CRUD（委托领域服务）
    // ========================================================================

    /// 获取配置列表
    pub async fn get_config_all(&self) -> AppResult<Vec<FileConfig>> {
        self.file_service.get_config_all().await
    }

    /// 根据 ID 获取配置
    pub async fn get_config(&self, id: i32) -> AppResult<FileConfig> {
        self.file_service.get_config(id).await
    }

    /// 创建配置
    pub async fn create_config(
        &self,
        name: String,
        storage: i32,
        remark: Option<String>,
        config: String,
        creator: Option<String>,
    ) -> AppResult<FileConfig> {
        self.file_service
            .create_config(name, storage, remark, config, creator)
            .await
    }

    /// 更新配置
    pub async fn update_config(
        &self,
        id: i32,
        name: String,
        storage: i32,
        remark: Option<String>,
        config: String,
        updater: Option<String>,
    ) -> AppResult<FileConfig> {
        self.file_service
            .update_config(id, name, storage, remark, config, updater)
            .await
    }

    /// 删除配置
    pub async fn delete_config(&self, id: i32, updater: Option<String>) -> AppResult<()> {
        self.file_service.delete_config(id, updater).await
    }

    /// 设为主配置（业务不变式由领域服务保证）
    pub async fn set_master_config(&self, id: i32, updater: Option<String>) -> AppResult<FileConfig> {
        self.file_service.set_master_config(id, updater).await
    }
}
