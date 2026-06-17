use std::sync::Arc;

use crate::file::dto::*;
use admin_domain::file::model::value_object::{FileQuery, FileUploadCommand};
use admin_domain::file::service::FileService;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct FileAppService {
    file_service: Arc<FileService>,
}

impl FileAppService {
    /// 创建文件应用服务实例
    ///
    /// # 参数
    /// * `file_service` - 文件领域服务，用于执行文件管理相关的业务逻辑
    pub fn new(file_service: Arc<FileService>) -> Self {
        Self { file_service }
    }

    /// 上传文件记录
    ///
    /// # 参数
    /// * `cmd` - 上传文件命令，包含文件名称、存储路径、访问URL、文件类型、文件大小、存储配置ID
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 将应用层DTO转换为领域层 `FileUploadCommand`
    /// 2. 委托给文件领域服务执行文件上传记录创建
    ///
    /// # 返回
    /// 成功返回 `FileResponse`，包含文件完整信息
    ///
    /// # 错误
    /// - 数据库写入异常
    pub async fn upload_file(
        &self,
        cmd: UploadFileCommand,
        creator: Option<String>,
    ) -> AppResult<FileResponse> {
        let upload_cmd = FileUploadCommand {
            name: cmd.name,
            path: cmd.path,
            url: cmd.url,
            file_type: cmd.file_type,
            size: cmd.size,
            config_id: cmd.config_id,
        };
        let file = self.file_service.upload_file(upload_cmd, creator).await?;
        Ok(FileResponse::from(file))
    }

    /// 删除文件记录
    ///
    /// # 参数
    /// * `file_id` - 要删除的文件ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给文件领域服务执行删除操作，逻辑详见 `FileService::delete_file`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundFile` - 文件ID对应的文件不存在
    /// - 数据库删除异常
    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        self.file_service.delete_file(file_id, updater).await
    }

    /// 分页查询文件列表
    ///
    /// # 参数
    /// * `request` - 分页查询请求，包含文件名称、文件类型、存储配置ID等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `FileQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给文件领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<FileResponse>`，包含文件列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_file_page(
        &self,
        request: FileQueryRequest,
    ) -> AppResult<Page<FileResponse>> {
        let query = FileQuery {
            name: request.name,
            file_type: request.file_type,
            config_id: request.config_id,
        };
        let page = Page::request(request.page, request.size);
        let result = self.file_service.get_file_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(FileResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 根据ID获取文件信息
    ///
    /// # 参数
    /// * `file_id` - 文件ID
    ///
    /// # 执行逻辑
    /// 委托给文件领域服务查询文件，逻辑详见 `FileService::get_file`
    ///
    /// # 返回
    /// 成功返回 `FileResponse`
    ///
    /// # 错误
    /// - `NotFoundFile` - 文件ID对应的文件不存在
    pub async fn get_file(&self, file_id: u64) -> AppResult<FileResponse> {
        let file = self.file_service.get_file(file_id).await?;
        Ok(FileResponse::from(file))
    }

    /// 获取文件下载信息
    ///
    /// # 参数
    /// * `file_id` - 文件ID
    ///
    /// # 执行逻辑
    /// 委托给文件领域服务获取文件下载信息，逻辑详见 `FileService::download_file`
    ///
    /// # 返回
    /// 成功返回 `FileDownloadResponse`，包含文件下载所需的详细信息
    ///
    /// # 错误
    /// - `NotFoundFile` - 文件ID对应的文件不存在
    /// - 文件存储访问异常
    pub async fn download_file(&self, file_id: u64) -> AppResult<FileDownloadResponse> {
        let info = self.file_service.download_file(file_id).await?;
        Ok(FileDownloadResponse::from(info))
    }
}
