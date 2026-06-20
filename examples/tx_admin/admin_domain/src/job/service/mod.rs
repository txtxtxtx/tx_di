use std::sync::Arc;
use tx_common::id;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;

use crate::shared::repository::RepositoryError;
use crate::job::model::aggregate::Job;
use crate::job::model::value_object::JobQuery;
use crate::job::repository::{JobRepository, JobLogRepository};

/// 定时任务领域服务
#[tx_comp]
pub struct JobService {
    job_repo: Arc<dyn JobRepository>,
    job_log_repo: Arc<dyn JobLogRepository>,
}

impl JobService {
    /// 创建 JobService 的新实例
    pub fn new(job_repo: Arc<dyn JobRepository>, job_log_repo: Arc<dyn JobLogRepository>) -> Self {
        Self { job_repo, job_log_repo }
    }

    /// 创建定时任务
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
        let job_id = id::next_id();
        let job = Job::create(
            job_id,
            name,
            handler_name,
            handler_param,
            cron_expression,
            retry_count,
            retry_interval,
            monitor_timeout,
            creator,
        );
        self.job_repo.insert(&job).await?;
        Ok(job)
    }

    /// 更新定时任务信息
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
        let mut job = self
            .job_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundJob)?;

        job.update_info(
            name,
            handler_name,
            handler_param,
            cron_expression,
            retry_count,
            retry_interval,
            monitor_timeout,
            updater,
        );
        self.job_repo.update(&job).await?;
        Ok(job)
    }

    /// 软删除定时任务
    pub async fn delete_job(
        &self,
        id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut job = self
            .job_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundJob)?;

        job.soft_delete(updater);
        self.job_repo.update(&job).await?;
        Ok(())
    }

    /// 根据 ID 获取定时任务详情
    pub async fn get_job(&self, id: u64) -> AppResult<Job> {
        Ok(self
            .job_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundJob)?)
    }

    /// 分页查询定时任务列表
    pub async fn get_job_page(
        &self,
        query: &JobQuery,
        page: Page<Job>,
    ) -> AppResult<Page<Job>> {
        self.job_repo.find_page(query, page).await
    }

    /// 获取所有运行中的定时任务（供调度器使用）
    pub async fn find_active_jobs(&self) -> AppResult<Vec<Job>> {
        self.job_repo.find_active().await
    }

    /// 变更定时任务状态（暂停/运行）
    pub async fn change_status(
        &self,
        id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<Job> {
        let mut job = self
            .job_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundJob)?;

        job.change_status(status, updater);
        self.job_repo.update(&job).await?;
        Ok(job)
    }
}
