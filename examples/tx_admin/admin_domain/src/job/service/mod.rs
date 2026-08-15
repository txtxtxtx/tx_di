//! 任务域领域服务
//!
//! 封装定时任务与执行日志的用例逻辑与不变量校验，
//! 只依赖 `JobRepository`（仓储 trait），不依赖基础设施插件。

use std::sync::Arc;

use tx_di_core::{Component, DepsTuple};
use tx_error::{AppError, AppResult};

use crate::job::model::aggregate::{Job, JobLog};
use crate::job::model::value_object::{ExecutionStatus, JobLogQuery, JobQuery, JobStatus};
use crate::job::repository::{JobRepository, JobRepositoryError};
use crate::shared::model::AuditFields;

/// 任务域领域服务
#[derive(Component)]
pub struct JobService {
    repo: Arc<dyn JobRepository>,
}

impl JobService {
    pub fn new(repo: Arc<dyn JobRepository>) -> Self {
        Self { repo }
    }

    /// 创建任务
    #[allow(clippy::too_many_arguments)]
    pub async fn create_job(
        &self,
        name: String,
        handler_name: String,
        handler_param: Option<String>,
        cron_expression: String,
        retry_count: i32,
        retry_interval: i32,
        monitor_timeout: i32,
        creator: Option<String>,
    ) -> AppResult<Job> {
        let now = jiff::Timestamp::now();
        let audit = AuditFields {
            creator: creator.clone(),
            create_time: now,
            updater: creator,
            update_time: now,
            ..Default::default()
        };
        let job = Job::new(
            tx_common::id::next_id(),
            name,
            JobStatus::Running,
            handler_name,
            handler_param,
            cron_expression,
            retry_count,
            retry_interval,
            monitor_timeout,
            audit,
        );
        if let Some(msg) = job.validate() {
            return Err(AppError::with_context(JobRepositoryError::ValidationJob, msg));
        }
        self.repo.create_job(&job).await
    }

    /// 更新任务信息
    #[allow(clippy::too_many_arguments)]
    pub async fn update_job(
        &self,
        id: u64,
        name: String,
        handler_name: String,
        handler_param: Option<String>,
        cron_expression: String,
        retry_count: i32,
        retry_interval: i32,
        monitor_timeout: i32,
        updater: Option<String>,
    ) -> AppResult<Job> {
        let mut job = self.repo.get_job_by_id(id).await?;
        job.name = name;
        job.handler_name = handler_name;
        job.handler_param = handler_param;
        job.cron_expression = cron_expression;
        job.retry_count = retry_count;
        job.retry_interval = retry_interval;
        job.monitor_timeout = monitor_timeout;
        job.audit.updater = updater;
        job.audit.update_time = jiff::Timestamp::now();

        if let Some(msg) = job.validate() {
            return Err(AppError::with_context(JobRepositoryError::ValidationJob, msg));
        }
        self.repo.update_job(&job).await
    }

    /// 删除任务（软删除）
    pub async fn delete_job(&self, id: u64) -> AppResult<()> {
        self.repo.delete_job(id).await
    }

    /// 按 ID 获取任务详情
    pub async fn get_job(&self, id: u64) -> AppResult<Job> {
        self.repo.get_job_by_id(id).await
    }

    /// 分页查询任务列表
    pub async fn get_job_page(&self, query: JobQuery) -> AppResult<(Vec<Job>, i64)> {
        self.repo.find_job_page(&query).await
    }

    /// 变更任务状态（暂停/运行）
    pub async fn change_status(&self, id: u64, status: i32, updater: Option<String>) -> AppResult<Job> {
        let mut job = self.repo.get_job_by_id(id).await?;
        job.status = match status {
            0 => JobStatus::Paused,
            _ => JobStatus::Running,
        };
        job.audit.updater = updater;
        job.audit.update_time = jiff::Timestamp::now();
        self.repo.update_job(&job).await
    }

    /// 分页查询任务执行日志
    pub async fn get_job_log_page(&self, query: JobLogQuery) -> AppResult<(Vec<JobLog>, i64)> {
        self.repo.find_job_log_page(&query).await
    }

    /// 按 ID 获取执行日志详情
    pub async fn get_job_log(&self, id: u64) -> AppResult<JobLog> {
        self.repo.get_job_log_by_id(id).await
    }

    /// 清空执行日志
    pub async fn clean_job_logs(&self, job_id: Option<u64>) -> AppResult<()> {
        self.repo.clean_job_logs(job_id).await
    }

    /// 创建执行日志（供应用层编排 `run_job` 时使用）
    pub async fn create_job_log(&self, log: &JobLog) -> AppResult<JobLog> {
        self.repo.create_job_log(log).await
    }

    /// 更新执行日志（供应用层编排 `run_job` 时使用）
    pub async fn update_job_log(&self, log: &JobLog) -> AppResult<JobLog> {
        self.repo.update_job_log(log).await
    }

    /// 结束执行日志：写入状态、结果与耗时
    pub async fn finish_job_log(
        &self,
        log_id: u64,
        status: ExecutionStatus,
        result: Option<String>,
        operator: Option<String>,
    ) -> AppResult<JobLog> {
        let mut log = self.repo.get_job_log_by_id(log_id).await?;
        log.status = status;
        log.result = result;
        log.end_time = Some(jiff::Timestamp::now());
        log.audit.updater = operator;
        log.audit.update_time = jiff::Timestamp::now();
        self.repo.update_job_log(&log).await
    }
}
