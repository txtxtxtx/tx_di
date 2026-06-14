use std::any::Any;
use async_trait::async_trait;

use crate::config::model::aggregate::Config;
use crate::config::model::value_object::ConfigQuery;
use tx_common::page::Page;
use tx_error::AppResult;

#[async_trait]
pub trait ConfigRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Config>>;
    async fn find_by_key(&self, key: &str) -> AppResult<Option<Config>>;
    async fn find_by_keys(&self, keys: &[String]) -> AppResult<Vec<Config>>;
    async fn find_page(
        &self,
        query: &ConfigQuery,
        page: Page<Config>,
    ) -> AppResult<Page<Config>>;
    async fn find_all(&self, query: &ConfigQuery) -> AppResult<Vec<Config>>;
    async fn insert(&self, config: &Config) -> AppResult<()>;
    async fn update(&self, config: &Config) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn exists_by_key(&self, key: &str) -> AppResult<bool>;
}
