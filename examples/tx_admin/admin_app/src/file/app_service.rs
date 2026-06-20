use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

use admin_domain::file::model::aggregate::FileConfig;
use admin_domain::file::model::value_object::{FileQuery, FileUploadCommand};
use admin_domain::file::service::FileService;
use admin_domain::file::repository::FileConfigRepository;
use admin_proto::{ListFilesRequest, FileResponse};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_file::storage::{guess_mime_type, extract_extension, FileStorageErr, FileStorage};
use tx_di_file::FilePlugin;
use tx_error::{AppError, AppResult};

use crate::file::dto::{file_to_response, DownloadFileStream};

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
    /// - `config_id` 为 Some 时查找指定配置
    /// - 为 None 时查找主配置（master=1）
    /// - DB 无配置时回退到 FilePlugin 的 TOML 配置
    async fn get_storage(&self, config_id: Option<i32>) -> AppResult<Arc<dyn FileStorage>> {
        // 尝试从 DB 获取配置
        let db_config = if let Some(cid) = config_id {
            self.file_config_repo.find_by_id(cid).await?
        } else {
            self.file_config_repo.find_master().await?
        };

        if let Some(cfg) = db_config {
            if cfg.storage == 2 {
                // Database 后端 — 由调用方处理，这里返回错误
                return Err(AppError::with_context(
                    FileStorageErr::NotFound,
                    "数据库存储后端不适用此操作",
                ));
            }
            if let Some(storage) = tx_di_file::create_storage(cfg.storage, &cfg.config) {
                return Ok(storage);
            }
        }

        // 回退到插件默认存储
        Ok(self.file_plugin.storage())
    }

    /// 获取文件配置（带校验）
    async fn get_config_or_err(&self, id: i32) -> AppResult<FileConfig> {
        self.file_config_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::with_context(FileStorageErr::NotFound, "文件配置不存在"))
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

        // 2. 校验扩展名（从 DB 或 TOML 配置获取白名单）
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

        // 9. 持久化元数据到 DB
        let cmd = FileUploadCommand {
            name: filename,
            path: storage_path,
            url,
            file_type: Some(mime),
            size: actual_size,
            config_id,
        };
        let file = self.file_service.upload_file(cmd, creator).await?;
        Ok(file_to_response(file))
    }

    /// 获取允许的文件扩展名列表
    async fn get_allowed_extensions(&self, config_id: Option<i32>) -> Vec<String> {
        let db_config = if let Some(cid) = config_id {
            self.file_config_repo.find_by_id(cid).await.ok().flatten()
        } else {
            self.file_config_repo.find_master().await.ok().flatten()
        };

        if let Some(cfg) = db_config {
            // 从 JSON config 中解析 allowed_extensions
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&cfg.config) {
                if let Some(arr) = json.get("allowed_extensions").and_then(|v| v.as_array()) {
                    return arr
                        .iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect();
                }
            }
        }

        // 回退到插件配置
        self.file_plugin.config.allowed_extensions.clone()
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
    // 文件配置 CRUD
    // ========================================================================

    /// 获取配置列表
    pub async fn get_config_all(&self) -> AppResult<Vec<FileConfig>> {
        self.file_config_repo.find_all().await
    }

    /// 根据 ID 获取配置
    pub async fn get_config(&self, id: i32) -> AppResult<FileConfig> {
        self.get_config_or_err(id).await
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
        let id = (jiff::Timestamp::now().as_millisecond() % i32::MAX as i64) as i32;
        let agg = FileConfig::create(id, name, storage, remark, config, creator);
        self.file_config_repo.insert(&agg).await?;
        Ok(agg)
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
        let mut agg = self.get_config_or_err(id).await?;
        agg.update_info(name, storage, remark, config, updater);
        self.file_config_repo.update(&agg).await?;
        Ok(agg)
    }

    /// 删除配置
    pub async fn delete_config(&self, id: i32, updater: Option<String>) -> AppResult<()> {
        let mut agg = self.get_config_or_err(id).await?;
        agg.soft_delete(updater);
        self.file_config_repo.update(&agg).await?;
        Ok(())
    }

    /// 设为主配置（先取消当前主配置，再设置新主配置）
    pub async fn set_master_config(&self, id: i32, updater: Option<String>) -> AppResult<FileConfig> {
        // 取消当前主配置
        if let Some(mut current_master) = self.file_config_repo.find_master().await? {
            if current_master.id != id {
                current_master.unset_master(updater.clone());
                self.file_config_repo.update(&current_master).await?;
            }
        }

        // 设置新主配置
        let mut agg = self.get_config_or_err(id).await?;
        agg.set_master(updater);
        self.file_config_repo.update(&agg).await?;
        Ok(agg)
    }
}
