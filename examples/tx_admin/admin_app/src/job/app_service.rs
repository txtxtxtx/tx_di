use std::sync::Arc;

use admin_domain::job::model::aggregate::JobLog;
use admin_domain::job::model::value_object::{ExecutionStatus, JobLogQuery, JobQuery};
use admin_domain::job::service::JobService;
use admin_domain::shared::model::AuditFields;
use admin_proto::{JobLogResponse, JobResponse};
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_di_job::{ExecutionStatus as InfrustExecutionStatus, JobPlugin};
use tx_error::AppResult;

use crate::job::dto::{
    CreateJobRequest, ListJobLogsRequest, ListJobsRequest, UpdateJobRequest, job_log_to_response,
    job_to_response,
};

/// 定时任务应用服务
///
/// 编排领域服务 `JobService`（用例逻辑）与 `JobPlugin`（调度执行），
/// 不再直接依赖数据访问层。
#[derive(Component)]
pub struct JobAppService {
    job_service: Arc<JobService>,
    job_plugin: Arc<JobPlugin>,
}

impl JobAppService {
    /// 创建定时任务应用服务实例（供集成测试手动构造）
    pub fn new(job_service: Arc<JobService>, job_plugin: Arc<JobPlugin>) -> Self {
        Self {
            job_service,
            job_plugin,
        }
    }

    /// 创建定时任务
    pub async fn create_job(
        &self,
        req: CreateJobRequest,
        creator: Option<String>,
    ) -> AppResult<JobResponse> {
        let job = self
            .job_service
            .create_job(
                req.name,
                req.handler_name,
                req.handler_param,
                req.cron_expression,
                req.retry_count,
                req.retry_interval,
                req.monitor_timeout,
                creator,
            )
            .await?;
        Ok(job_to_response(&job))
    }

    /// 更新定时任务信息
    pub async fn update_job(
        &self,
        req: UpdateJobRequest,
        updater: Option<String>,
    ) -> AppResult<JobResponse> {
        let job = self
            .job_service
            .update_job(
                req.id,
                req.name,
                req.handler_name,
                req.handler_param,
                req.cron_expression,
                req.retry_count,
                req.retry_interval,
                req.monitor_timeout,
                updater,
            )
            .await?;
        Ok(job_to_response(&job))
    }

    /// 删除定时任务（软删除）
    pub async fn delete_job(&self, id: u64, _updater: Option<String>) -> AppResult<()> {
        self.job_service.delete_job(id).await
    }

    /// 根据 ID 获取定时任务详情
    pub async fn get_job(&self, id: u64) -> AppResult<JobResponse> {
        let job = self.job_service.get_job(id).await?;
        Ok(job_to_response(&job))
    }

    /// 分页查询定时任务列表（SQL 层过滤 + 分页）
    pub async fn get_job_page(&self, req: ListJobsRequest) -> AppResult<Page<JobResponse>> {
        let query = JobQuery {
            name: req.name,
            status: req.status,
            page: req.page,
            page_size: req.page_size,
        };
        let (rows, total) = self.job_service.get_job_page(query).await?;
        let list: Vec<JobResponse> = rows.iter().map(job_to_response).collect();
        Ok(Page::new(list, req.page, req.page_size, total))
    }

    /// 变更定时任务状态（暂停/运行）
    pub async fn change_status(
        &self,
        id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<JobResponse> {
        let job = self.job_service.change_status(id, status, updater).await?;
        Ok(job_to_response(&job))
    }

    /// 分页查询任务执行日志（SQL 层过滤 + 分页）
    pub async fn get_job_log_page(
        &self,
        req: ListJobLogsRequest,
    ) -> AppResult<Page<JobLogResponse>> {
        let query = JobLogQuery {
            job_id: req.job_id,
            status: req.status,
            page: req.page,
            page_size: req.page_size,
        };
        let (rows, total) = self.job_service.get_job_log_page(query).await?;
        let list: Vec<JobLogResponse> = rows.iter().map(job_log_to_response).collect();
        Ok(Page::new(list, req.page, req.page_size, total))
    }

    /// 根据 ID 获取任务执行日志详情
    pub async fn get_job_log(&self, id: u64) -> AppResult<JobLogResponse> {
        let log = self.job_service.get_job_log(id).await?;
        Ok(job_log_to_response(&log))
    }

    /// 清空任务执行日志
    pub async fn clean_job_logs(&self, job_id: Option<u64>) -> AppResult<()> {
        self.job_service.clean_job_logs(job_id).await
    }

    /// 手动执行定时任务
    pub async fn run_job(&self, id: u64, operator: Option<String>) -> AppResult<()> {
        // 1. 获取任务
        let job = self.job_service.get_job(id).await?;

        // 2. 创建执行日志（开始执行）
        let now = jiff::Timestamp::now();
        let log = JobLog::new(
            tx_common::id::next_id(),
            job.id,
            job.handler_name.clone(),
            job.handler_param.clone(),
            1,
            now,
            None,
            None,
            ExecutionStatus::Failed,
            None,
            AuditFields {
                creator: operator.clone(),
                create_time: now,
                updater: operator.clone(),
                update_time: now,
                ..Default::default()
            },
        );
        let log = self.job_service.create_job_log(&log).await?;

        // 3. 通过 JobPlugin 执行
        let result = self
            .job_plugin
            .execute_by_type(id, &job.handler_name, job.handler_param.as_deref())
            .await;

        // 4. 更新执行日志
        let (status, result_msg) = if result.status == InfrustExecutionStatus::Success {
            (ExecutionStatus::Success, result.result)
        } else {
            (
                ExecutionStatus::Failed,
                result.error.or(Some("执行失败".to_string())),
            )
        };
        self.job_service
            .finish_job_log(log.id, status, result_msg, operator)
            .await?;
        Ok(())
    }
}
