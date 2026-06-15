use std::sync::Arc;

use crate::file::model::aggregate::File;
use crate::file::model::value_object::{FileDownloadInfo, FileQuery, FileUploadCommand};
use crate::file::repository::{FileConfigRepository, FileRepository};
use crate::shared::repository::RepositoryError::NotFound;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::id;

#[tx_comp]
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
    ) -> AppResult<File> {
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

    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        let mut file = self
            .file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| NotFound)?;

        file.soft_delete(updater);
        self.file_repo.update(&file).await?;
        Ok(())
    }

    pub async fn get_file_page(
        &self,
        query: &FileQuery,
        page: Page<File>,
    ) -> AppResult<Page<File>> {
        self.file_repo.find_page(query, page).await
    }

    pub async fn get_file(&self, file_id: u64) -> AppResult<File> {
        Ok(self.file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| NotFound)?)
    }

    pub async fn download_file(&self, file_id: u64) -> AppResult<FileDownloadInfo> {
        let file = self.file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| NotFound)?;

        // Determine MIME type from file extension
        let content_type = match file.name.rsplit('.').next() {
            Some("pdf") => "application/pdf",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("txt") => "text/plain",
            Some("html" | "htm") => "text/html",
            Some("css") => "text/css",
            Some("js") => "application/javascript",
            Some("json") => "application/json",
            Some("xml") => "application/xml",
            Some("zip") => "application/zip",
            Some("doc" | "docx") => "application/msword",
            Some("xls" | "xlsx") => "application/vnd.ms-excel",
            _ => "application/octet-stream",
        };

        Ok(FileDownloadInfo {
            url: file.url,
            filename: file.name,
            size: file.size,
            content_type: content_type.to_string(),
        })
    }
}

#[cfg(test)]
mod tests;
