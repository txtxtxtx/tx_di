use std::any::Any;
use async_trait::async_trait;

use crate::file::model::aggregate::{File, FileConfig};
use crate::file::model::value_object::FileQuery;
use tx_common::page::Page;
use tx_error::AppResult;

#[async_trait]
pub trait FileRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<File>>;
    async fn find_page(
        &self,
        query: &FileQuery,
        page: Page<File>,
    ) -> AppResult<Page<File>>;
    async fn insert(&self, file: &File) -> AppResult<()>;
    async fn update(&self, file: &File) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn find_file_path(&self, id: u64) -> AppResult<String>;
}

#[async_trait]
pub trait FileConfigRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<FileConfig>>;
    async fn find_master(&self) -> AppResult<Option<FileConfig>>;
    async fn find_all(&self) -> AppResult<Vec<FileConfig>>;
    async fn insert(&self, config: &FileConfig) -> AppResult<()>;
    async fn update(&self, config: &FileConfig) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
}
