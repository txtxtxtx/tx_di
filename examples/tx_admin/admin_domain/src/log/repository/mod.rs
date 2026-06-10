use async_trait::async_trait;

use crate::log::model::aggregate::{LoginLog, OperateLog};
use crate::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

#[async_trait]
pub trait OperateLogRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<OperateLog>, RepositoryError>;
    async fn find_page(
        &self,
        query: &OperateLogQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<OperateLog>, RepositoryError>;
    async fn insert(&self, log: &OperateLog) -> Result<(), RepositoryError>;
    async fn delete_by_ids(&self, ids: &[u64]) -> Result<(), RepositoryError>;
    async fn clean_all(&self) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait LoginLogRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<LoginLog>, RepositoryError>;
    async fn find_page(
        &self,
        query: &LoginLogQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<LoginLog>, RepositoryError>;
    async fn insert(&self, log: &LoginLog) -> Result<(), RepositoryError>;
    async fn delete_by_ids(&self, ids: &[u64]) -> Result<(), RepositoryError>;
    async fn clean_all(&self) -> Result<(), RepositoryError>;
}
