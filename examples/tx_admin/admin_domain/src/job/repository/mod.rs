use std::any::Any;
use async_trait::async_trait;

use crate::job::model::aggregate::{Job, JobLog};
use crate::job::model::value_object::{JobQuery, JobLogQuery};
use tx_common::page::Page;
use tx_error::AppResult;

#[async_trait]
pub trait JobRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Job>>;
    async fn find_active(&self) -> AppResult<Vec<Job>>;
    async fn find_page(
        &self,
        query: &JobQuery,
        page: Page<Job>,
    ) -> AppResult<Page<Job>>;
    async fn insert(&self, job: &Job) -> AppResult<()>;
    async fn update(&self, job: &Job) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
}

#[async_trait]
pub trait JobLogRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<JobLog>>;
    async fn find_page(
        &self,
        query: &JobLogQuery,
        page: Page<JobLog>,
    ) -> AppResult<Page<JobLog>>;
    async fn insert(&self, log: &JobLog) -> AppResult<()>;
    async fn update(&self, log: &JobLog) -> AppResult<()>;
    async fn clean_by_job_id(&self, job_id: Option<u64>) -> AppResult<()>;
}
