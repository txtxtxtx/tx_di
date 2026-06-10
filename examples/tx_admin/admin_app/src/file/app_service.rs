use std::sync::Arc;

use crate::file::dto::*;
use admin_domain::file::model::value_object::{FileQuery, FileUploadCommand};
use admin_domain::file::service::FileService;
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

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
    ) -> Result<FileResponse, RepositoryError> {
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

    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        self.file_service.delete_file(file_id, updater).await
    }

    pub async fn get_file_page(
        &self,
        request: FileQueryRequest,
    ) -> Result<PageResponse<FileResponse>, RepositoryError> {
        let query = FileQuery {
            name: request.name,
            file_type: request.file_type,
            config_id: request.config_id,
        };
        let page = PageRequest::new(request.page, request.page_size);
        let result = self.file_service.get_file_page(&query, &page).await?;

        Ok(PageResponse::new(
            result.list.into_iter().map(FileResponse::from).collect(),
            result.total,
            result.page,
            result.page_size,
        ))
    }

    pub async fn get_file(&self, file_id: u64) -> Result<FileResponse, RepositoryError> {
        let file = self.file_service.get_file(file_id).await?;
        Ok(FileResponse::from(file))
    }
}
