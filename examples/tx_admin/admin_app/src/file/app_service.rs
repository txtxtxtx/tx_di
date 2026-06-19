use std::sync::Arc;

use tokio::io::AsyncRead;

use admin_domain::file::model::value_object::{FileQuery, FileUploadCommand};
use admin_domain::file::service::FileService;
use admin_proto::{ListFilesRequest, FileResponse};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_file::storage::{guess_mime_type, extract_extension, FileStorageErr};
use tx_di_file::FilePlugin;
use tx_error::{AppError, AppResult};

use crate::file::dto::{file_to_response, DownloadFileStream};
use crate::file::limited_reader::LimitedAsyncRead;

#[tx_comp]
pub struct FileAppService {
    pub file_service: Arc<FileService>,
    pub file_plugin: Arc<FilePlugin>,
}

impl FileAppService {
    pub fn new(file_service: Arc<FileService>, file_plugin: Arc<FilePlugin>) -> Self {
        Self {
            file_service,
            file_plugin,
        }
    }

    // ========================================================================
    // 流式上传（核心新方法）
    // ========================================================================

    /// 流式上传单个文件 —— 零内存缓冲
    ///
    /// 文件二进制通过 `AsyncRead` 传入，流经 `LimitedAsyncRead`（大小拦截），
    /// 直接写入存储后端，全程不将文件内容加载到应用内存。
    ///
    /// # 流程
    /// 1. 校验扩展名 → `FileConfig::allowed_extensions`
    /// 2. `LimitedAsyncRead` 包装 reader（边读边计数，超限即断）
    /// 3. 生成存储路径 `YYYY/MM/uuid_filename.ext`
    /// 4. `storage.write_stream(path, &mut limited, content_type)`
    /// 5. 写 DB 元数据 → 返回 `FileResponse`
    pub async fn upload_file_stream(
        &self,
        filename: String,
        content_type: String,
        reader: &mut (dyn AsyncRead + Unpin + Send),
        config_id: Option<i32>,
        creator: Option<String>,
    ) -> AppResult<FileResponse> {
        let storage = self.file_plugin.storage();
        let config = &self.file_plugin.config;

        // 1. 校验扩展名
        if !config.allowed_extensions.is_empty() {
            let ext = extract_extension(&filename).unwrap_or_default();
            if !config.allowed_extensions.contains(&ext) {
                return Err(AppError::with_context(
                    FileStorageErr::InvalidExtension,
                    format!("不允许的文件类型: .{}", ext),
                ));
            }
        }

        // 2. 大小感知流式拦截
        let max_size = if config.max_file_size > 0 {
            config.max_file_size
        } else {
            u64::MAX
        };
        let mut limited = LimitedAsyncRead::new(reader, max_size);

        // 3. 生成存储路径: YYYY/MM/uuid_filename.ext
        let now = jiff::Zoned::now();
        let file_uuid = uuid::Uuid::now_v7();
        let path = format!(
            "{:04}/{:02}/{}_{}",
            now.year(),
            now.month(),
            file_uuid,
            filename
        );

        // 4. 流式写入存储后端
        let storage_path = storage
            .write_stream(&path, &mut limited, Some(&content_type))
            .await?;

        // 5. 获取实际写入大小
        let actual_size = limited.bytes_read() as i32;

        // 6. 推断 MIME（multipart 可能传 application/octet-stream）
        let mime = if content_type.is_empty() || content_type == "application/octet-stream" {
            guess_mime_type(&filename)
        } else {
            content_type
        };

        // 7. 生成访问 URL（优先 presigned，本地存储回退到公开 URL）
        let url = storage
            .presigned_url(&storage_path, 3600)
            .await
            .unwrap_or_else(|_| storage_path.clone());

        // 8. 持久化元数据到 DB
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

    // ========================================================================
    // 流式下载
    // ========================================================================

    /// 流式下载文件 —— 返回 `AsyncRead`，不缓冲到内存
    ///
    /// # 流程
    /// 1. 查 DB 获取文件元数据（包含存储路径）
    /// 2. `storage.read_stream(path)` → `AsyncRead`
    /// 3. 返回 `DownloadFileStream`（含 reader + 元数据，供 API handler 构造 HTTP 响应）
    pub async fn download_file_stream(&self, file_id: u64) -> AppResult<DownloadFileStream> {
        let file = self.file_service.get_file(file_id).await?;
        let storage = self.file_plugin.storage();
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
    // 删除（增强：物理文件 + DB 软删除）
    // ========================================================================

    /// 删除文件 —— 先删物理文件，再 DB 软删除
    ///
    /// 物理文件删除失败仅 warn，不阻塞 DB 软删除。
    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        // 1. 先获取文件信息（查存储路径）
        let file = self.file_service.get_file(file_id).await?;

        // 2. 删除物理文件（失败不阻塞）
        let storage = self.file_plugin.storage();
        if let Err(e) = storage.delete(&file.path).await {
            tracing::warn!(
                file_id = file_id,
                path = %file.path,
                error = %e,
                "删除物理文件失败，继续执行 DB 软删除"
            );
        }

        // 3. DB 软删除
        self.file_service.delete_file(file_id, updater).await
    }

    // ========================================================================
    // 查询（保持不变）
    // ========================================================================

    /// 分页查询文件列表
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

    /// 根据 ID 获取文件信息
    pub async fn get_file(&self, file_id: u64) -> AppResult<FileResponse> {
        let file = self.file_service.get_file(file_id).await?;
        Ok(file_to_response(file))
    }
}
