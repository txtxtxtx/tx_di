//! 任务域仓储 trait

use std::any::Any;

use async_trait::async_trait;
use tx_error::{AppResult, CodeMsg};

use crate::job::model::aggregate::{Job, JobLog};
use crate::job::model::value_object::{JobLogQuery, JobQuery};

/// Job 仓储错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum JobRepositoryError {
    #[err(10010, "数据库异常")]
    DatabaseJob,
    #[err(10110, "记录不存在")]
    NotFoundJob,
    #[err(10312, "任务参数不合法")]
    ValidationJob,
}

/// Job repository trait
#[async_trait]
pub trait JobRepository: Any + Send + Sync {
    /// 创建任务
    async fn create_job(&self, job: &Job) -> AppResult<Job>;

    /// 更新任务（全字段覆盖）
    async fn update_job(&self, job: &Job) -> AppResult<Job>;

    /// 软删除任务
    async fn delete_job(&self, id: u64) -> AppResult<()>;

    /// 按 ID 查询任务（排除已删除）
    async fn get_job_by_id(&self, id: u64) -> AppResult<Job>;

    /// 分页查询任务（SQL 层过滤 + COUNT），返回 `(列表, 总数)`
    async fn find_job_page(&self, query: &JobQuery) -> AppResult<(Vec<Job>, i64)>;

    /// 创建执行日志
    async fn create_job_log(&self, log: &JobLog) -> AppResult<JobLog>;

    /// 更新执行日志
    async fn update_job_log(&self, log: &JobLog) -> AppResult<JobLog>;

    /// 按 ID 查询执行日志
    async fn get_job_log_by_id(&self, id: u64) -> AppResult<JobLog>;

    /// 分页查询执行日志（SQL 层过滤 + COUNT），返回 `(列表, 总数)`
    async fn find_job_log_page(&self, query: &JobLogQuery) -> AppResult<(Vec<JobLog>, i64)>;

    /// 清空执行日志（软删除），`job_id=None` 表示清空所有
    async fn clean_job_logs(&self, job_id: Option<u64>) -> AppResult<()>;
}
