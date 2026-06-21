use std::sync::Arc;
use tx_error::{AppError, AppResult};
use crate::err::JobErr;
use crate::models::{InfrustJob, InfrustJobLog, JobStatus, AuditFields, SoftDelete};
use tx_di_toasty::ToastyPlugin;
use toasty::stmt::Query;

/// toasty::Error → AppError 辅助转换
#[inline]
fn to_err(e: toasty::Error) -> AppError {
    AppError::Internal(e.into())
}

/// 任务数据访问层
pub struct JobRepository {
    tp: Arc<ToastyPlugin>,
}

impl JobRepository {
    pub fn new(tp: Arc<ToastyPlugin>) -> Self {
        Self { tp }
    }

    /// 创建任务
    pub async fn create_job(&self, job: InfrustJob) -> AppResult<InfrustJob> {
        let mut db = self.tp.db().clone();
        let created = InfrustJob::create()
            .id(job.id)
            .name(job.name)
            .status(job.status)
            .handler_name(job.handler_name)
            .handler_param(job.handler_param)
            .cron_expression(job.cron_expression)
            .retry_count(job.retry_count)
            .retry_interval(job.retry_interval)
            .monitor_timeout(job.monitor_timeout)
            .audit(job.audit)
            .soft_delete(job.soft_delete)
            .exec(&mut db)
            .await
            .map_err(to_err)?;
        tracing::info!(job_id = created.id, "创建任务成功");
        Ok(created)
    }

    /// 更新任务（全字段覆盖）
    /// `exec()` 返回 `()`，因此直接返回传入的 job（其字段已是最新值）
    pub async fn update_job(&self, job: InfrustJob) -> AppResult<InfrustJob> {
        let mut db = self.tp.db().clone();
        let mut existing = InfrustJob::get_by_id(&mut db, job.id).await.map_err(to_err)?;

        existing
            .update()
            .name(job.name.clone())
            .status(job.status)
            .handler_name(job.handler_name.clone())
            .handler_param(job.handler_param.clone())
            .cron_expression(job.cron_expression.clone())
            .retry_count(job.retry_count)
            .retry_interval(job.retry_interval)
            .monitor_timeout(job.monitor_timeout)
            .audit(job.audit.clone())
            .soft_delete(job.soft_delete)
            .exec(&mut db)
            .await
            .map_err(to_err)?;
        tracing::info!(job_id = job.id, "更新任务成功");
        Ok(job)
    }

    /// 软删除任务
    pub async fn delete_job(&self, job_id: i64) -> AppResult<()> {
        let mut db = self.tp.db().clone();
        match InfrustJob::get_by_id(&mut db, job_id).await {
            Ok(mut job) => {
                job.update()
                    .soft_delete(SoftDelete::DELETED)
                    .exec(&mut db)
                    .await
                    .map_err(to_err)?;
                tracing::info!(job_id = job_id, "删除任务成功");
            }
            Err(e) => {
                return Err(to_err(e));
            }
        }
        Ok(())
    }

    /// 按 ID 查询任务（排除已删除）
    pub async fn get_job_by_id(&self, job_id: i64) -> AppResult<InfrustJob> {
        let mut db = self.tp.db().clone();
        let job = InfrustJob::get_by_id(&mut db, job_id).await.map_err(to_err)?;
        if job.is_deleted() {
            return Err(AppError::with_context(
                JobErr::JobNotFound,
                format!("id={}", job_id),
            ));
        }
        Ok(job)
    }

    /// 查询所有运行中的任务（id 倒序，排除已删除，供调度器使用）
    pub async fn get_all_running_jobs(&self) -> AppResult<Vec<InfrustJob>> {
        let mut db = self.tp.db().clone();
        let mut query = Query::<toasty::stmt::List<InfrustJob>>::all()
            .and(InfrustJob::fields().status().eq(JobStatus::Running))
            .and(InfrustJob::fields().soft_delete().eq(SoftDelete::NORMAL));
        query.order_by(InfrustJob::fields().id().desc());
        let jobs = query.exec(&mut db).await.map_err(to_err)?;
        Ok(jobs)
    }

    /// 分页查询运行中的任务（id 倒序，排除已删除）
    ///
    /// - `page`: 页码（从 1 开始）
    /// - `page_size`: 每页条数
    pub async fn get_running_jobs(&self, page: tx_common::Page<InfrustJob>) -> AppResult<Vec<InfrustJob>> {
        let mut db = self.tp.db().clone();
        let offset = page.offset() as usize;

        let mut query = Query::<toasty::stmt::List<InfrustJob>>::all()
            .and(InfrustJob::fields().status().eq(JobStatus::Running))
            .and(InfrustJob::fields().soft_delete().eq(SoftDelete::NORMAL));
        query.order_by(InfrustJob::fields().id().desc());
        query.limit(page.size as usize);
        query.offset(offset);

        let jobs = query.exec(&mut db).await.map_err(to_err)?;
        Ok(jobs)
    }

    /// 创建执行日志
    pub async fn create_job_log(&self, log: InfrustJobLog) -> AppResult<InfrustJobLog> {
        let mut db = self.tp.db().clone();
        let created = InfrustJobLog::create()
            .id(log.id)
            .job_id(log.job_id)
            .handler_name(log.handler_name)
            .handler_param(log.handler_param)
            .execute_index(log.execute_index)
            .begin_time(log.begin_time)
            .end_time(log.end_time)
            .duration(log.duration)
            .status(log.status)
            .result(log.result)
            .audit(log.audit)
            .soft_delete(log.soft_delete)
            .exec(&mut db)
            .await
            .map_err(to_err)?;
        Ok(created)
    }

    /// 更新执行日志
    /// `exec()` 返回 `()`，因此直接返回传入的 log（其字段已是最新值）
    pub async fn update_job_log(&self, log: InfrustJobLog) -> AppResult<InfrustJobLog> {
        let mut db = self.tp.db().clone();
        let mut existing = InfrustJobLog::get_by_id(&mut db, log.id).await.map_err(to_err)?;

        existing
            .update()
            .end_time(log.end_time)
            .duration(log.duration)
            .status(log.status)
            .result(log.result.clone())
            .audit(log.audit.clone())
            .soft_delete(log.soft_delete)
            .exec(&mut db)
            .await
            .map_err(to_err)?;
        Ok(log)
    }

    /// 分页查询任务的执行日志（id 倒序，全部在数据库层完成）
    ///
    /// - `job_id`: 任务 ID
    /// - `page`: 分页参数
    pub async fn get_job_logs(&self, job_id: i64, page: tx_common::page::Page<InfrustJobLog>) -> AppResult<Vec<InfrustJobLog>> {
        let mut db = self.tp.db().clone();
        let offset = page.offset() as usize;

        let mut query = Query::<toasty::stmt::List<InfrustJobLog>>::all()
            .and(InfrustJobLog::fields().job_id().eq(job_id))
            .and(InfrustJobLog::fields().soft_delete().eq(SoftDelete::NORMAL));
        query.order_by(InfrustJobLog::fields().id().desc());
        query.limit(page.size as usize);
        query.offset(offset);

        let logs = query.exec(&mut db).await.map_err(to_err)?;
        Ok(logs)
    }

    /// 查询所有未删除的任务（id 倒序）
    pub async fn get_all_jobs(&self) -> AppResult<Vec<InfrustJob>> {
        let mut db = self.tp.db().clone();
        let mut query = Query::<toasty::stmt::List<InfrustJob>>::all()
            .and(InfrustJob::fields().soft_delete().eq(SoftDelete::NORMAL));
        query.order_by(InfrustJob::fields().id().desc());
        query.exec(&mut db).await.map_err(to_err)
    }

    /// 按 ID 查询执行日志
    pub async fn get_job_log_by_id(&self, log_id: i64) -> AppResult<InfrustJobLog> {
        let mut db = self.tp.db().clone();
        let log = InfrustJobLog::get_by_id(&mut db, log_id).await.map_err(to_err)?;
        if log.soft_delete != SoftDelete::NORMAL {
            return Err(AppError::with_context(
                JobErr::JobNotFound,
                format!("日志 id={} 已删除", log_id),
            ));
        }
        Ok(log)
    }

    /// 清空执行日志（软删除），按 job_id 过滤，None 表示清空所有
    pub async fn clean_job_logs(&self, job_id: Option<i64>) -> AppResult<()> {
        let mut db = self.tp.db().clone();
        let mut query = Query::<toasty::stmt::List<InfrustJobLog>>::all()
            .and(InfrustJobLog::fields().soft_delete().eq(SoftDelete::NORMAL));
        if let Some(jid) = job_id {
            query = query.and(InfrustJobLog::fields().job_id().eq(jid));
        }
        let logs = query.exec(&mut db).await.map_err(to_err)?;

        let now = jiff::Timestamp::now().to_string();
        for log in logs {
            let mut existing = InfrustJobLog::get_by_id(&mut db, log.id).await.map_err(to_err)?;
            let old_audit = existing.audit.clone();
            existing
                .update()
                .soft_delete(SoftDelete::DELETED)
                .audit(AuditFields {
                    creator: old_audit.creator,
                    create_time: old_audit.create_time,
                    updater: Some("system".to_string()),
                    update_time: now.clone(),
                })
                .exec(&mut db)
                .await
                .map_err(to_err)?;
        }
        Ok(())
    }

    /// 查询所有未删除的执行日志（id 倒序），可选择性按 job_id 过滤
    pub async fn get_all_job_logs(&self, job_id: Option<i64>) -> AppResult<Vec<InfrustJobLog>> {
        let mut db = self.tp.db().clone();
        let mut query = Query::<toasty::stmt::List<InfrustJobLog>>::all()
            .and(InfrustJobLog::fields().soft_delete().eq(SoftDelete::NORMAL));
        if let Some(jid) = job_id {
            query = query.and(InfrustJobLog::fields().job_id().eq(jid));
        }
        query.order_by(InfrustJobLog::fields().id().desc());
        query.exec(&mut db).await.map_err(to_err)
    }
}
