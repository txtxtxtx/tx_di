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
    pub fn new(file_service: Arc<FileService>) -> Self {
        Self { file_service }
    }

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

    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        self.file_service.delete_file(file_id, updater).await
    }

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

    pub async fn get_file(&self, file_id: u64) -> AppResult<FileResponse> {
        let file = self.file_service.get_file(file_id).await?;
        Ok(FileResponse::from(file))
    }

    pub async fn download_file(&self, file_id: u64) -> AppResult<FileDownloadResponse> {
        let info = self.file_service.download_file(file_id).await?;
        Ok(FileDownloadResponse::from(info))
    }
}
