//! Job 仓储实现
//!
//! 实现 domain 层的 `JobRepository` trait，内部适配 `tx_di_job` 插件的数据访问层，
//! 完成领域模型 `Job`/`JobLog` 与插件模型 `InfrustJob`/`InfrustJobLog` 之间的双向转换。
//!
//! 错误处理：直接透传 `tx_di_job` 插件返回的 `AppError`（其错误码语义保持不变，
//! 避免前端 i18n 回归），不在本层重新映射错误码。

use std::sync::Arc;

use admin_domain::job::model::aggregate::{Job, JobLog};
use admin_domain::job::model::value_object::{
    ExecutionStatus as DomainExecutionStatus, JobLogQuery, JobQuery, JobStatus as DomainJobStatus,
};
use admin_domain::job::repository::JobRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use async_trait::async_trait;
use tx_di_core::{Component, DepsTuple};
use tx_di_job::{
    AuditFields as InfrustAuditFields, ExecutionStatus as InfrustExecutionStatus, InfrustJob,
    InfrustJobLog, JobStatus as InfrustJobStatus, SoftDelete as InfrustSoftDelete,
};
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

fn to_infrust_status(s: DomainJobStatus) -> InfrustJobStatus {
    match s {
        DomainJobStatus::Paused => InfrustJobStatus::Paused,
        DomainJobStatus::Running => InfrustJobStatus::Running,
    }
}

fn from_infrust_status(s: InfrustJobStatus) -> DomainJobStatus {
    match s {
        InfrustJobStatus::Paused => DomainJobStatus::Paused,
        InfrustJobStatus::Running => DomainJobStatus::Running,
    }
}

fn to_infrust_exec_status(s: DomainExecutionStatus) -> InfrustExecutionStatus {
    match s {
        DomainExecutionStatus::Failed => InfrustExecutionStatus::Failed,
        DomainExecutionStatus::Success => InfrustExecutionStatus::Success,
        DomainExecutionStatus::Timeout => InfrustExecutionStatus::Timeout,
        DomainExecutionStatus::Retrying => InfrustExecutionStatus::Retrying,
    }
}

fn from_infrust_exec_status(s: InfrustExecutionStatus) -> DomainExecutionStatus {
    match s {
        InfrustExecutionStatus::Failed => DomainExecutionStatus::Failed,
        InfrustExecutionStatus::Success => DomainExecutionStatus::Success,
        InfrustExecutionStatus::Timeout => DomainExecutionStatus::Timeout,
        InfrustExecutionStatus::Retrying => DomainExecutionStatus::Retrying,
    }
}

fn to_infrust_job(job: &Job) -> InfrustJob {
    let deleted = job.audit.is_deleted();
    InfrustJob {
        id: job.id,
        name: job.name.clone(),
        status: to_infrust_status(job.status),
        handler_name: job.handler_name.clone(),
        handler_param: job.handler_param.clone(),
        cron_expression: job.cron_expression.clone(),
        retry_count: job.retry_count,
        retry_interval: job.retry_interval,
        monitor_timeout: job.monitor_timeout,
        audit: InfrustAuditFields {
            creator: job.audit.creator.clone(),
            create_time: job.audit.create_time,
            updater: job.audit.updater.clone(),
            update_time: job.audit.update_time,
        },
        soft_delete: if deleted {
            InfrustSoftDelete::DELETED
        } else {
            InfrustSoftDelete::NORMAL
        },
    }
}

fn from_infrust_job(job: &InfrustJob) -> Job {
    Job::new(
        job.id,
        job.name.clone(),
        from_infrust_status(job.status),
        job.handler_name.clone(),
        job.handler_param.clone(),
        job.cron_expression.clone(),
        job.retry_count,
        job.retry_interval,
        job.monitor_timeout,
        AuditFields {
            creator: job.audit.creator.clone(),
            create_time: job.audit.create_time,
            updater: job.audit.updater.clone(),
            update_time: job.audit.update_time,
            deleted: if job.soft_delete == InfrustSoftDelete::DELETED {
                DeletedStatus::Deleted
            } else {
                DeletedStatus::Normal
            },
        },
    )
}

fn to_infrust_job_log(log: &JobLog) -> InfrustJobLog {
    let deleted = log.audit.is_deleted();
    InfrustJobLog {
        id: log.id,
        job_id: log.job_id,
        handler_name: log.handler_name.clone(),
        handler_param: log.handler_param.clone(),
        execute_index: log.execute_index,
        begin_time: log.begin_time,
        end_time: log.end_time,
        duration: log.duration,
        status: to_infrust_exec_status(log.status),
        result: log.result.clone(),
        audit: InfrustAuditFields {
            creator: log.audit.creator.clone(),
            create_time: log.audit.create_time,
            updater: log.audit.updater.clone(),
            update_time: log.audit.update_time,
        },
        soft_delete: if deleted {
            InfrustSoftDelete::DELETED
        } else {
            InfrustSoftDelete::NORMAL
        },
    }
}

fn from_infrust_job_log(log: &InfrustJobLog) -> JobLog {
    JobLog::new(
        log.id,
        log.job_id,
        log.handler_name.clone(),
        log.handler_param.clone(),
        log.execute_index,
        log.begin_time,
        log.end_time,
        log.duration,
        from_infrust_exec_status(log.status),
        log.result.clone(),
        AuditFields {
            creator: log.audit.creator.clone(),
            create_time: log.audit.create_time,
            updater: log.audit.updater.clone(),
            update_time: log.audit.update_time,
            deleted: if log.soft_delete == InfrustSoftDelete::DELETED {
                DeletedStatus::Deleted
            } else {
                DeletedStatus::Normal
            },
        },
    )
}

/// Job 仓储实现（适配 tx_di_job 插件）
#[derive(Component)]
#[component(as_trait = dyn JobRepository)]
pub struct ToastyJobRepository {
    tp: Arc<ToastyPlugin>,
}

impl ToastyJobRepository {
    pub fn new(tp: Arc<ToastyPlugin>) -> Self {
        Self { tp }
    }

    fn repo(&self) -> tx_di_job::JobRepository {
        tx_di_job::JobRepository::new(self.tp.clone())
    }
}

#[async_trait]
impl JobRepository for ToastyJobRepository {
    async fn create_job(&self, job: &Job) -> AppResult<Job> {
        let created = self.repo().create_job(to_infrust_job(job)).await?;
        Ok(from_infrust_job(&created))
    }

    async fn update_job(&self, job: &Job) -> AppResult<Job> {
        let updated = self.repo().update_job(to_infrust_job(job)).await?;
        Ok(from_infrust_job(&updated))
    }

    async fn delete_job(&self, id: u64) -> AppResult<()> {
        self.repo().delete_job(id).await
    }

    async fn get_job_by_id(&self, id: u64) -> AppResult<Job> {
        let job = self.repo().get_job_by_id(id).await?;
        Ok(from_infrust_job(&job))
    }

    async fn find_job_page(&self, query: &JobQuery) -> AppResult<(Vec<Job>, i64)> {
        let (rows, total) = self
            .repo()
            .find_job_page(query.name.as_deref(), query.status, query.page, query.page_size)
            .await?;
        Ok((rows.iter().map(from_infrust_job).collect(), total))
    }

    async fn create_job_log(&self, log: &JobLog) -> AppResult<JobLog> {
        let created = self.repo().create_job_log(to_infrust_job_log(log)).await?;
        Ok(from_infrust_job_log(&created))
    }

    async fn update_job_log(&self, log: &JobLog) -> AppResult<JobLog> {
        let updated = self.repo().update_job_log(to_infrust_job_log(log)).await?;
        Ok(from_infrust_job_log(&updated))
    }

    async fn get_job_log_by_id(&self, id: u64) -> AppResult<JobLog> {
        let log = self.repo().get_job_log_by_id(id).await?;
        Ok(from_infrust_job_log(&log))
    }

    async fn find_job_log_page(&self, query: &JobLogQuery) -> AppResult<(Vec<JobLog>, i64)> {
        let (rows, total) = self
            .repo()
            .find_job_log_page(query.job_id, query.status, query.page, query.page_size)
            .await?;
        Ok((rows.iter().map(from_infrust_job_log).collect(), total))
    }

    async fn clean_job_logs(&self, job_id: Option<u64>) -> AppResult<()> {
        self.repo().clean_job_logs(job_id).await
    }
}
