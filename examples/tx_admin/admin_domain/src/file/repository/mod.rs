use async_trait::async_trait;

use crate::file::model::aggregate::{File, FileConfig};
use crate::file::model::value_object::FileQuery;
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<File>, RepositoryError>;
    async fn find_page(
        &self,
        query: &FileQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<File>, RepositoryError>;
    async fn insert(&self, file: &File) -> Result<(), RepositoryError>;
    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait FileConfigRepository: Send + Sync {
    async fn find_by_id(&self, id: i32) -> Result<Option<FileConfig>, RepositoryError>;
    async fn find_master(&self) -> Result<Option<FileConfig>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<FileConfig>, RepositoryError>;
    async fn insert(&self, config: &FileConfig) -> Result<(), RepositoryError>;
    async fn update(&self, config: &FileConfig) -> Result<(), RepositoryError>;
    async fn soft_delete(&self, id: i32) -> Result<(), RepositoryError>;
}
