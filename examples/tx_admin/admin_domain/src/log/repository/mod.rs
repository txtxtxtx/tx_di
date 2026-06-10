use async_trait::async_trait;

use crate::log::model::aggregate::{LoginLog, OperateLog};
use crate::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use tx_common::page::Page;
use tx_error::AppResult;

#[async_trait]
pub trait OperateLogRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<OperateLog>>;
    async fn find_page(
        &self,
        query: &OperateLogQuery,
        page: Page<OperateLog>,
    ) -> AppResult<Page<OperateLog>>;
    async fn insert(&self, log: &OperateLog) -> AppResult<()>;
    async fn delete_by_ids(&self, ids: &[u64]) -> AppResult<()>;
    async fn clean_all(&self) -> AppResult<()>;
}

#[async_trait]
pub trait LoginLogRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<LoginLog>>;
    async fn find_page(
        &self,
        query: &LoginLogQuery,
        page: Page<LoginLog>,
    ) -> AppResult<Page<LoginLog>>;
    async fn insert(&self, log: &LoginLog) -> AppResult<()>;
    async fn delete_by_ids(&self, ids: &[u64]) -> AppResult<()>;
    async fn clean_all(&self) -> AppResult<()>;
}
