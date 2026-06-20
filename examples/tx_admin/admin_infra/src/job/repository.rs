use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::job::model::aggregate::{Job, JobLog};
use admin_domain::job::model::value_object::{JobQuery, JobLogQuery};
use admin_domain::job::repository::{JobRepository, JobLogRepository};
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::{RepositoryError, db_err};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::{SysJob, SysJobLog};
use crate::common::Deleted;

/// Toasty impl JobRepository
#[tx_comp(as_trait = dyn JobRepository)]
pub struct ToastyJobRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyJobRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }
    fn to_domain(m: &SysJob) -> Job {
        Job::restore(
            m.id as u64,
            m.name.clone(),
            m.status,
            m.handler_name.clone(),
            if m.handler_param.is_empty() { None } else { Some(m.handler_param.clone()) },
            m.cron_expression.clone(),
            m.retry_count,
            m.retry_interval,
            m.monitor_timeout,
            AuditFields {
                creator: if m.creator.is_empty() { None } else { Some(m.creator.clone()) },
                create_time: m.created_at.parse().unwrap_or_default(),
                updater: if m.updater.is_empty() { None } else { Some(m.updater.clone()) },
                update_time: m.updated_at.parse().unwrap_or_default(),
                deleted: if m.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl JobRepository for ToastyJobRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Job>> {
        let mut db = self.plugin.db().clone();
        match SysJob::get_by_id(&mut db, id as i64).await {
            Ok(m) if m.deleted == Deleted::No => Ok(Some(Self::to_domain(&m))),
            _ => Ok(None),
        }
    }

    async fn find_active(&self) -> AppResult<Vec<Job>> {
        let mut db = self.plugin.db().clone();
        let all = SysJob::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJob))?;

        Ok(all
            .iter()
            .filter(|m| m.deleted == Deleted::No && m.status == 1)
            .map(Self::to_domain)
            .collect())
    }

    async fn find_page(&self, query: &JobQuery, page: Page<Job>) -> AppResult<Page<Job>> {
        let mut db = self.plugin.db().clone();
        let all = SysJob::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJob))?;

        let filtered: Vec<&SysJob> = all
            .iter()
            .filter(|m| m.deleted == Deleted::No)
            .filter(|m| {
                if let Some(ref name) = query.name {
                    if !m.name.contains(name.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if m.status != status { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<Job> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, job: &Job) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysJob::create()
            .id(job.id as i64)
            .name(job.name.clone())
            .status(job.status)
            .handler_name(job.handler_name.clone())
            .handler_param(job.handler_param.clone().unwrap_or_default())
            .cron_expression(job.cron_expression.clone())
            .retry_count(job.retry_count)
            .retry_interval(job.retry_interval)
            .monitor_timeout(job.monitor_timeout)
            .creator(job.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(job.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(job.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJob))?;
        Ok(())
    }

    async fn update(&self, job: &Job) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysJob::get_by_id(&mut db, job.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundJob)?;
        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(job.name.clone())
            .status(job.status)
            .handler_name(job.handler_name.clone())
            .handler_param(job.handler_param.clone().unwrap_or_default())
            .cron_expression(job.cron_expression.clone())
            .retry_count(job.retry_count)
            .retry_interval(job.retry_interval)
            .monitor_timeout(job.monitor_timeout)
            .updater(job.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(job.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJob))?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut job = SysJob::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundJob)?;

        job.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJob))?;
        Ok(())
    }
}

/// Toasty impl JobLogRepository
#[tx_comp(as_trait = dyn JobLogRepository)]
pub struct ToastyJobLogRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyJobLogRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(m: &SysJobLog) -> JobLog {
        JobLog::restore(
            m.id as u64,
            m.job_id as u64,
            m.handler_name.clone(),
            if m.handler_param.is_empty() { None } else { Some(m.handler_param.clone()) },
            m.execute_index,
            m.begin_time.clone(),
            if m.end_time.is_empty() { None } else { Some(m.end_time.clone()) },
            if m.duration == 0 { None } else { Some(m.duration) },
            m.status,
            if m.result.is_empty() { None } else { Some(m.result.clone()) },
            AuditFields {
                creator: if m.creator.is_empty() { None } else { Some(m.creator.clone()) },
                create_time: m.created_at.parse().unwrap_or_default(),
                updater: if m.updater.is_empty() { None } else { Some(m.updater.clone()) },
                update_time: m.updated_at.parse().unwrap_or_default(),
                deleted: if m.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl JobLogRepository for ToastyJobLogRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<JobLog>> {
        let mut db = self.plugin.db().clone();
        match SysJobLog::get_by_id(&mut db, id as i64).await {
            Ok(m) if m.deleted == Deleted::No => Ok(Some(Self::to_domain(&m))),
            _ => Ok(None),
        }
    }

    async fn find_page(&self, query: &JobLogQuery, page: Page<JobLog>) -> AppResult<Page<JobLog>> {
        let mut db = self.plugin.db().clone();
        let all = SysJobLog::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJobLog))?;

        let filtered: Vec<&SysJobLog> = all
            .iter()
            .filter(|m| m.deleted == Deleted::No)
            .filter(|m| {
                if let Some(job_id) = query.job_id {
                    if m.job_id != job_id as i64 { return false; }
                }
                if let Some(status) = query.status {
                    if m.status != status { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<JobLog> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn insert(&self, log: &JobLog) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysJobLog::create()
            .id(log.id as i64)
            .job_id(log.job_id as i64)
            .handler_name(log.handler_name.clone())
            .handler_param(log.handler_param.clone().unwrap_or_default())
            .execute_index(log.execute_index)
            .begin_time(log.begin_time.clone())
            .end_time(log.end_time.clone().unwrap_or_default())
            .duration(log.duration.unwrap_or(0))
            .status(log.status)
            .result(log.result.clone().unwrap_or_default())
            .creator(log.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(log.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(log.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJobLog))?;
        Ok(())
    }

    async fn update(&self, log: &JobLog) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysJobLog::get_by_id(&mut db, log.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFoundJobLog)?;
        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .job_id(log.job_id as i64)
            .handler_name(log.handler_name.clone())
            .handler_param(log.handler_param.clone().unwrap_or_default())
            .execute_index(log.execute_index)
            .begin_time(log.begin_time.clone())
            .end_time(log.end_time.clone().unwrap_or_default())
            .duration(log.duration.unwrap_or(0))
            .status(log.status)
            .result(log.result.clone().unwrap_or_default())
            .updater(log.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(log.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJobLog))?;
        Ok(())
    }

    async fn clean_by_job_id(&self, job_id: Option<u64>) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let all = SysJobLog::all()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseJobLog))?;

        let now = jiff::Timestamp::now().to_string();
        for m in all.iter().filter(|m| m.deleted == Deleted::No) {
            if let Some(jid) = job_id {
                if m.job_id != jid as i64 { continue; }
            }
            let mut log = SysJobLog::get_by_id(&mut db, m.id)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseJobLog))?;
            log.update()
                .deleted(Deleted::Yes)
                .updated_at(now.clone())
                .exec(&mut db)
                .await
                .map_err(|e| db_err(e, RepositoryError::DatabaseJobLog))?;
        }

        Ok(())
    }
}
