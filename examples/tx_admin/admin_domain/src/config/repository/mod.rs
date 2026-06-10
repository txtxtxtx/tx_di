use async_trait::async_trait;

use crate::config::model::aggregate::Config;
use crate::config::model::value_object::ConfigQuery;
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Config>, RepositoryError>;
    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, RepositoryError>;
    async fn find_page(
        &self,
        query: &ConfigQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<Config>, RepositoryError>;
    async fn find_all(&self, query: &ConfigQuery) -> Result<Vec<Config>, RepositoryError>;
    async fn insert(&self, config: &Config) -> Result<(), RepositoryError>;
    async fn update(&self, config: &Config) -> Result<(), RepositoryError>;
    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError>;
    async fn exists_by_key(&self, key: &str) -> Result<bool, RepositoryError>;
}
