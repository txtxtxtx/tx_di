use std::sync::Arc;

use admin_proto::{UploadFileRequest, ListFilesRequest, FileResponse, DownloadFileResponse};
use admin_domain::file::model::value_object::{FileQuery, FileUploadCommand};
use admin_domain::file::service::FileService;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

use crate::empty_string::opt_filter;
use crate::file::dto::{file_to_response, file_download_to_response};

#[tx_comp]
pub struct FileAppService {
    file_service: Arc<FileService>,
}

impl FileAppService {
    /// 创建文件应用服务实例
    pub fn new(file_service: Arc<FileService>) -> Self {
        Self { file_service }
    }

    /// 上传文件记录
    pub async fn upload_file(
        &self,
        req: UploadFileRequest,
        creator: Option<String>,
    ) -> AppResult<FileResponse> {
        let upload_cmd = FileUploadCommand {
            name: req.name,
            path: req.path,
            url: req.url,
            file_type: opt_filter(req.file_type),
            size: req.size,
            config_id: req.config_id,
        };
        let file = self.file_service.upload_file(upload_cmd, creator).await?;
        Ok(file_to_response(file))
    }

    /// 删除文件记录
    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        self.file_service.delete_file(file_id, updater).await
    }

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

    /// 根据ID获取文件信息
    pub async fn get_file(&self, file_id: u64) -> AppResult<FileResponse> {
        let file = self.file_service.get_file(file_id).await?;
        Ok(file_to_response(file))
    }

    /// 获取文件下载信息
    pub async fn download_file(&self, file_id: u64) -> AppResult<DownloadFileResponse> {
        let info = self.file_service.download_file(file_id).await?;
        Ok(file_download_to_response(info))
    }
}
