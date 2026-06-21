use std::sync::Arc;

use admin_proto::{JobResponse, JobLogResponse};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_job::{JobPlugin, JobRepository, InfrustJob, InfrustJobLog, JobStatus, AuditFields, SoftDelete, ExecutionStatus};
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use crate::job::dto::{CreateJobRequest, UpdateJobRequest, ListJobsRequest, ListJobLogsRequest, job_to_response, job_log_to_response};

/// 定时任务应用服务
///
/// 基于 tx_di_job 插件，封装定时任务和任务日志的用例逻辑。
#[tx_comp]
pub struct JobAppService {
    tp: Arc<ToastyPlugin>,
    job_plugin: Arc<JobPlugin>,
}

impl JobAppService {
    fn repo(&self) -> JobRepository {
        JobRepository::new(self.tp.clone())
    }

    /// 创建定时任务
    pub async fn create_job(
        &self,
        req: CreateJobRequest,
        creator: Option<String>,
    ) -> AppResult<JobResponse> {
        let now = jiff::Timestamp::now().to_string();
        let job_id = tx_common::id::next_id() as i64;

        let job = InfrustJob {
            id: job_id,
            name: req.name,
            status: JobStatus::Running,
            handler_name: req.handler_name,
            handler_param: req.handler_param,
            cron_expression: req.cron_expression,
            retry_count: req.retry_count,
            retry_interval: req.retry_interval,
            monitor_timeout: req.monitor_timeout,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: now.clone(),
                updater: creator,
                update_time: now,
            },
            soft_delete: SoftDelete::NORMAL,
        };

        let created = self.repo().create_job(job).await?;
        Ok(job_to_response(created))
    }

    /// 更新定时任务信息
    pub async fn update_job(
        &self,
        req: UpdateJobRequest,
        updater: Option<String>,
    ) -> AppResult<JobResponse> {
        let mut job = self.repo().get_job_by_id(req.id as i64).await?;
        let now = jiff::Timestamp::now().to_string();

        job.name = req.name;
        job.handler_name = req.handler_name;
        job.handler_param = req.handler_param;
        job.cron_expression = req.cron_expression;
        job.retry_count = req.retry_count;
        job.retry_interval = req.retry_interval;
        job.monitor_timeout = req.monitor_timeout;
        job.audit.updater = updater;
        job.audit.update_time = now;

        let updated = self.repo().update_job(job).await?;
        Ok(job_to_response(updated))
    }

    /// 删除定时任务（软删除）
    pub async fn delete_job(&self, id: u64, _updater: Option<String>) -> AppResult<()> {
        self.repo().delete_job(id as i64).await
    }

    /// 根据 ID 获取定时任务详情
    pub async fn get_job(&self, id: u64) -> AppResult<JobResponse> {
        let job = self.repo().get_job_by_id(id as i64).await?;
        Ok(job_to_response(job))
    }

    /// 分页查询定时任务列表
    pub async fn get_job_page(&self, req: ListJobsRequest) -> AppResult<Page<JobResponse>> {
        let all = self.repo().get_all_jobs().await?;

        // 内存筛选
        let filtered: Vec<InfrustJob> = all
            .into_iter()
            .filter(|j| {
                if let Some(ref name) = req.name {
                    if !j.name.contains(name.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = req.status {
                    if (j.status as i32) != status {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = ((req.page - 1) * req.page_size).max(0) as usize;
        let size = req.page_size as usize;

        let list: Vec<JobResponse> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(job_to_response)
            .collect();

        Ok(Page::new(list, req.page, req.page_size, total))
    }

    /// 变更定时任务状态（暂停/运行）
    pub async fn change_status(
        &self,
        id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<JobResponse> {
        let mut job = self.repo().get_job_by_id(id as i64).await?;
        let now = jiff::Timestamp::now().to_string();

        job.status = match status {
            0 => JobStatus::Paused,
            _ => JobStatus::Running,
        };
        job.audit.updater = updater;
        job.audit.update_time = now;

        let updated = self.repo().update_job(job).await?;
        Ok(job_to_response(updated))
    }

    /// 分页查询任务执行日志
    pub async fn get_job_log_page(&self, req: ListJobLogsRequest) -> AppResult<Page<JobLogResponse>> {
        let all = self.repo().get_all_job_logs(req.job_id.map(|id| id as i64)).await?;

        // 内存筛选
        let filtered: Vec<InfrustJobLog> = all
            .into_iter()
            .filter(|l| {
                if let Some(status) = req.status {
                    if (l.status as i32) != status {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = ((req.page - 1) * req.page_size).max(0) as usize;
        let size = req.page_size as usize;

        let list: Vec<JobLogResponse> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(job_log_to_response)
            .collect();

        Ok(Page::new(list, req.page, req.page_size, total))
    }

    /// 根据 ID 获取任务执行日志详情
    pub async fn get_job_log(&self, id: u64) -> AppResult<JobLogResponse> {
        let log = self.repo().get_job_log_by_id(id as i64).await?;
        Ok(job_log_to_response(log))
    }

    /// 清空任务执行日志
    pub async fn clean_job_logs(&self, job_id: Option<u64>) -> AppResult<()> {
        self.repo().clean_job_logs(job_id.map(|id| id as i64)).await
    }

    /// 手动执行定时任务
    pub async fn run_job(&self, id: u64, operator: Option<String>) -> AppResult<()> {
        let job_id = id as i64;

        // 1. 获取任务
        let job = self.repo().get_job_by_id(job_id).await?;

        // 2. 创建执行日志（开始执行）
        let now = jiff::Timestamp::now().to_string();
        let log_id = tx_common::id::next_id() as i64;
        let log = InfrustJobLog {
            id: log_id,
            job_id,
            handler_name: job.handler_name.clone(),
            handler_param: job.handler_param.clone(),
            execute_index: 1,
            begin_time: now.clone(),
            end_time: None,
            duration: None,
            status: ExecutionStatus::Failed,
            result: None,
            audit: AuditFields {
                creator: operator.clone(),
                create_time: now.clone(),
                updater: operator.clone(),
                update_time: now,
            },
            soft_delete: SoftDelete::NORMAL,
        };
        self.repo().create_job_log(log).await?;

        // 3. 通过 JobPlugin 执行
        let result = self
            .job_plugin
            .execute_by_type(job_id, &job.handler_name, job.handler_param.as_deref())
            .await;

        // 4. 更新执行日志
        let mut log = self.repo().get_job_log_by_id(log_id).await?;
        let end_time = jiff::Timestamp::now().to_string();

        if result.status == ExecutionStatus::Success {
            log.status = ExecutionStatus::Success;
            log.result = result.result;
        } else {
            log.status = ExecutionStatus::Failed;
            log.result = result.error.or(Some("执行失败".to_string()));
        }
        log.end_time = Some(end_time);
        log.audit.updater = operator;
        log.audit.update_time = jiff::Timestamp::now().to_string();

        self.repo().update_job_log(log).await?;
        Ok(())
    }
}
