use std::sync::Arc;

use admin_domain::job::model::value_object::{JobQuery, JobLogQuery};
use admin_domain::job::repository::JobLogRepository;
use admin_domain::job::service::JobService;
use admin_proto::{JobResponse, JobLogResponse};
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_job::JobPlugin;
use tx_error::AppResult;

use crate::job::dto::{*, job_to_response, job_log_to_response};

/// 定时任务应用服务
///
/// 封装定时任务和任务日志的用例逻辑，协调领域服务、JobPlugin 与仓储完成业务操作。
#[tx_comp]
pub struct JobAppService {
    job_service: Arc<JobService>,
    job_log_repo: Arc<dyn JobLogRepository>,
    job_plugin: Arc<JobPlugin>,
}

impl JobAppService {
    pub fn new(
        job_service: Arc<JobService>,
        job_log_repo: Arc<dyn JobLogRepository>,
        job_plugin: Arc<JobPlugin>,
    ) -> Self {
        Self { job_service, job_log_repo, job_plugin }
    }

    /// 创建定时任务
    ///
    /// # 参数
    /// * `req` - 创建请求，包含任务名称、处理器名称、参数、Cron 表达式、重试配置
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 返回
    /// 成功返回 `JobResponse`
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
        Ok(job_to_response(job))
    }

    /// 更新定时任务信息
    ///
    /// # 参数
    /// * `req` - 更新请求
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 返回
    /// 成功返回更新后的 `JobResponse`
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
        Ok(job_to_response(job))
    }

    /// 删除定时任务
    ///
    /// # 参数
    /// * `id` - 任务ID
    /// * `updater` - 操作者标识（可选）
    pub async fn delete_job(&self, id: u64, updater: Option<String>) -> AppResult<()> {
        self.job_service.delete_job(id, updater).await
    }

    /// 根据ID获取定时任务详情
    ///
    /// # 参数
    /// * `id` - 任务ID
    pub async fn get_job(&self, id: u64) -> AppResult<JobResponse> {
        let job = self.job_service.get_job(id).await?;
        Ok(job_to_response(job))
    }

    /// 分页查询定时任务列表
    ///
    /// # 参数
    /// * `req` - 分页查询请求，包含名称、状态筛选及分页参数
    pub async fn get_job_page(&self, req: ListJobsRequest) -> AppResult<Page<JobResponse>> {
        let query = JobQuery {
            name: req.name,
            status: req.status,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.job_service.get_job_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(job_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 变更定时任务状态（暂停/运行）
    ///
    /// # 参数
    /// * `id` - 任务ID
    /// * `status` - 目标状态值（0=暂停, 1=运行）
    /// * `updater` - 操作者标识（可选）
    pub async fn change_status(
        &self,
        id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<JobResponse> {
        let job = self.job_service.change_status(id, status, updater).await?;
        Ok(job_to_response(job))
    }

    /// 分页查询任务执行日志
    ///
    /// # 参数
    /// * `req` - 分页查询请求，包含任务ID、状态筛选及分页参数
    pub async fn get_job_log_page(&self, req: ListJobLogsRequest) -> AppResult<Page<JobLogResponse>> {
        let query = JobLogQuery {
            job_id: req.job_id,
            status: req.status,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.job_log_repo.find_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(job_log_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 根据ID获取任务执行日志详情
    ///
    /// # 参数
    /// * `id` - 日志ID
    pub async fn get_job_log(&self, id: u64) -> AppResult<JobLogResponse> {
        let log = self
            .job_log_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| admin_domain::shared::repository::RepositoryError::NotFoundJobLog)?;
        Ok(job_log_to_response(log))
    }

    /// 清空任务执行日志
    ///
    /// # 参数
    /// * `job_id` - 任务ID，为空则清空所有日志
    pub async fn clean_job_logs(&self, job_id: Option<u64>) -> AppResult<()> {
        self.job_log_repo.clean_by_job_id(job_id).await
    }

    /// 手动执行定时任务
    ///
    /// # 参数
    /// * `id` - 任务ID
    pub async fn run_job(&self, id: u64) -> AppResult<()> {
        use admin_domain::job::model::aggregate::JobLog;
        use tx_common::id;
        use tx_di_job::ExecutionStatus;

        // 1. 获取任务
        let job = self.job_service.get_job(id).await?;

        // 2. 创建执行日志（开始执行）
        let log_id = id::next_id();
        let mut log = JobLog::create(
            log_id,
            job.id,
            job.handler_name.clone(),
            job.handler_param.clone(),
            1, // execute_index
            Some("manual".to_string()),
        );
        self.job_log_repo.insert(&log).await?;

        // 3. 通过 JobPlugin 执行
        let result = self
            .job_plugin
            .execute_by_type(job.id as i64, &job.handler_name, job.handler_param.as_deref())
            .await;

        // 4. 更新执行日志
        match result.status {
            ExecutionStatus::Success => {
                log.finish_success(
                    result.result.unwrap_or_default(),
                    Some("manual".to_string()),
                );
            }
            _ => {
                log.finish_failure(
                    result
                        .error
                        .unwrap_or_else(|| "执行失败".to_string()),
                    Some("manual".to_string()),
                );
            }
        }
        self.job_log_repo.update(&log).await?;

        Ok(())
    }
}
