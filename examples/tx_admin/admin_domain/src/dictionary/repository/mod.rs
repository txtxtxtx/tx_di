use async_trait::async_trait;

use crate::dictionary::model::aggregate::{DictData, DictType};
use crate::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use crate::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_error::AppResult;

#[async_trait]
pub trait DictTypeRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictType>>;
    async fn find_by_type(&self, dict_type: &str) -> AppResult<Option<DictType>>;
    async fn find_page(
        &self,
        query: &DictTypeQuery,
        page: Page<DictType>,
    ) -> AppResult<Page<DictType>>;
    async fn find_all(&self, query: &DictTypeQuery) -> AppResult<Vec<DictType>>;
    async fn insert(&self, dict_type: &DictType) -> AppResult<()>;
    async fn update(&self, dict_type: &DictType) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn exists_by_type(&self, dict_type: &str) -> AppResult<bool>;
}

#[async_trait]
pub trait DictDataRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<DictData>>;
    async fn find_by_type(&self, dict_type: &str) -> AppResult<Vec<DictData>>;
    async fn find_page(
        &self,
        query: &DictDataQuery,
        page: Page<DictData>,
    ) -> AppResult<Page<DictData>>;
    async fn insert(&self, data: &DictData) -> AppResult<()>;
    async fn update(&self, data: &DictData) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
}
