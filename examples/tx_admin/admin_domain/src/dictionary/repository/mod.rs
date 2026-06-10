use async_trait::async_trait;

use crate::dictionary::model::aggregate::{DictData, DictType};
use crate::dictionary::model::value_object::{DictDataQuery, DictTypeQuery};
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

#[async_trait]
pub trait DictTypeRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<DictType>, RepositoryError>;
    async fn find_by_type(&self, dict_type: &str) -> Result<Option<DictType>, RepositoryError>;
    async fn find_page(
        &self,
        query: &DictTypeQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<DictType>, RepositoryError>;
    async fn find_all(&self, query: &DictTypeQuery) -> Result<Vec<DictType>, RepositoryError>;
    async fn insert(&self, dict_type: &DictType) -> Result<(), RepositoryError>;
    async fn update(&self, dict_type: &DictType) -> Result<(), RepositoryError>;
    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError>;
    async fn exists_by_type(&self, dict_type: &str) -> Result<bool, RepositoryError>;
}

#[async_trait]
pub trait DictDataRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<DictData>, RepositoryError>;
    async fn find_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, RepositoryError>;
    async fn find_page(
        &self,
        query: &DictDataQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<DictData>, RepositoryError>;
    async fn insert(&self, data: &DictData) -> Result<(), RepositoryError>;
    async fn update(&self, data: &DictData) -> Result<(), RepositoryError>;
    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError>;
}
