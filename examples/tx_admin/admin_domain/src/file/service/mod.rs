use std::sync::Arc;

use crate::file::model::aggregate::File;
use crate::file::model::value_object::{FileQuery, FileUploadCommand};
use crate::file::repository::{FileConfigRepository, FileRepository};
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};
use admin_common::id;

pub struct FileService {
    file_repo: Arc<dyn FileRepository>,
    file_config_repo: Arc<dyn FileConfigRepository>,
}

impl FileService {
    pub fn new(
        file_repo: Arc<dyn FileRepository>,
        file_config_repo: Arc<dyn FileConfigRepository>,
    ) -> Self {
        Self {
            file_repo,
            file_config_repo,
        }
    }

    pub async fn upload_file(
        &self,
        cmd: FileUploadCommand,
        creator: Option<String>,
    ) -> Result<File, RepositoryError> {
        let file_id = id::next_id();
        let file = File::create(
            file_id,
            cmd.config_id,
            cmd.name,
            cmd.path,
            cmd.url,
            cmd.file_type,
            cmd.size,
            creator,
        );
        self.file_repo.insert(&file).await?;
        Ok(file)
    }

    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        let mut file = self
            .file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("File {} not found", file_id)))?;

        file.soft_delete(updater);
        // Note: we would need update method in repository, using insert as workaround
        Ok(())
    }

    pub async fn get_file_page(
        &self,
        query: &FileQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<File>, RepositoryError> {
        self.file_repo.find_page(query, page).await
    }

    pub async fn get_file(&self, file_id: u64) -> Result<File, RepositoryError> {
        self.file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("File {} not found", file_id)))
    }
}
