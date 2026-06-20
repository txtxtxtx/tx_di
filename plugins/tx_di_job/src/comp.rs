use std::sync::{Arc, OnceLock};
use tokio_util::sync::CancellationToken;
use tx_di_core::{App, CompInit, RIE, tx_comp, async_method, InnerContext};
use tracing::{info, warn, error, debug};
use jiff::Timestamp;

use crate::config::JobConfig;
use crate::models::{InfrustJob, InfrustJobLog, AuditFields, SoftDelete, ExecutionStatus, JobStatus};
use crate::err::JobResult;
use crate::repository::JobRepository;
use crate::executors::{JobExecutor, ExecutorType, InternalJobExecutor, ShellJobExecutor, PythonJobExecutor};
use tx_di_toasty::ToastyPlugin;
use tx_common::page::Page;

/// Job 插件组件
///
/// 封装定时任务调度功能，包括：
/// - Cron 表达式解析和调度
/// - 多种任务执行器（内部函数、Shell、Python）
/// - 任务执行日志记录
/// - 任务重试机制和超时监控
///
/// # DI 注入方式
///
/// ```rust,ignore
/// // 在其他组件中注入
/// #[tx_comp(init)]
/// pub struct MyService {
///     pub job_plugin: Arc<JobPlugin>,  // 自动注入
/// }
/// ```
///
/// # 注册内部任务处理器
///
/// ```rust,ignore
/// let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;
/// let job_plugin = app.inject::<JobPlugin>();
/// job_plugin.register_handler("send_email", |param| {
///     // 发送邮件逻辑
///     JobResult {
///         status: ExecutionStatus::Success,
///         result: Some("邮件发送成功".to_string()),
///         error: None,
///     }
/// });
/// ```
#[tx_comp(init)]
pub struct JobPlugin {
    /// 配置引用（DI 注入）
    pub config: Arc<JobConfig>,

    /// 数据库访问层（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub repository: OnceLock<Arc<JobRepository>>,

    /// 内部函数执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub internal_executor: OnceLock<Arc<InternalJobExecutor>>,

    /// Shell 脚本执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub shell_executor: OnceLock<Arc<ShellJobExecutor>>,

    /// Python 脚本执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub python_executor: OnceLock<Arc<PythonJobExecutor>>,

    /// 调度器句柄
    #[tx_cst(OnceLock::new())]
    pub scheduler_handle: OnceLock<tokio::task::JoinHandle<()>>,
}

impl JobPlugin {
    // ---- 辅助访问器 ----

    fn repo(&self) -> &Arc<JobRepository> {
        self.repository
            .get()
            .expect("JobRepository 未初始化，请确保 async_init 已完成")
    }

    fn internal_exec(&self) -> &Arc<InternalJobExecutor> {
        self.internal_executor
            .get()
            .expect("InternalJobExecutor 未初始化")
    }

    fn shell_exec(&self) -> &Arc<ShellJobExecutor> {
        self.shell_executor
            .get()
            .expect("ShellJobExecutor 未初始化")
    }

    fn python_exec(&self) -> &Arc<PythonJobExecutor> {
        self.python_executor
            .get()
            .expect("PythonJobExecutor 未初始化")
    }

    // ---- 公共 API ----

    /// 注册内部任务处理器
    pub fn register_handler<F>(&self, name: &str, handler: F)
    where
        F: Fn(Option<&str>) -> JobResult + Send + Sync + 'static,
    {
        self.internal_exec().register(name, handler);
    }

    /// 注销内部任务处理器
    pub fn unregister_handler(&self, name: &str) {
        self.internal_exec().unregister(name);
    }

    /// 手动触发任务执行
    pub async fn trigger_job(&self, job_id: i64) -> RIE<()> {
        info!(job_id = job_id, "手动触发任务执行");

        let job = self.repo().get_job_by_id(job_id).await?;
        self.execute_job(&job).await?;

        Ok(())
    }

    /// 创建任务
    pub async fn create_job(
        &self,
        name: &str,
        handler_name: &str,
        cron_expression: &str,
    ) -> RIE<InfrustJob> {
        info!(name = name, "创建任务");

        self.validate_cron_expression(cron_expression)?;

        let now = Timestamp::now().to_string();
        let job_id = tx_common::id::next_id() as i64;

        let job = InfrustJob {
            id: job_id,
            name: name.to_string(),
            status: JobStatus::Running,
            handler_name: handler_name.to_string(),
            handler_param: None,
            cron_expression: cron_expression.to_string(),
            retry_count: 0,
            retry_interval: 0,
            monitor_timeout: 0,
            audit: AuditFields {
                creator: Some("system".to_string()),
                create_time: now.clone(),
                updater: Some("system".to_string()),
                update_time: now,
            },
            soft_delete: SoftDelete::NORMAL,
        };

        let job = self.repo().create_job(job).await?;

        info!(job_id = job.id, "任务创建成功");
        Ok(job)
    }

    /// 更新任务
    pub async fn update_job(
        &self,
        job_id: i64,
        name: Option<&str>,
        handler_name: Option<&str>,
        cron_expression: Option<&str>,
    ) -> RIE<InfrustJob> {
        info!(job_id = job_id, "更新任务");

        let mut job = self.repo().get_job_by_id(job_id).await?;

        if let Some(name) = name {
            job.name = name.to_string();
        }

        if let Some(handler_name) = handler_name {
            job.handler_name = handler_name.to_string();
        }

        if let Some(cron_expression) = cron_expression {
            self.validate_cron_expression(cron_expression)?;
            job.cron_expression = cron_expression.to_string();
        }

        job.audit.update_time = Timestamp::now().to_string();

        let job = self.repo().update_job(job).await?;

        info!(job_id = job.id, "任务更新成功");
        Ok(job)
    }

    /// 删除任务（软删除）
    pub async fn delete_job(&self, job_id: i64) -> RIE<()> {
        info!(job_id = job_id, "删除任务");

        self.repo().delete_job(job_id).await?;

        info!(job_id = job_id, "任务删除成功");
        Ok(())
    }

    /// 暂停任务
    pub async fn pause_job(&self, job_id: i64) -> RIE<()> {
        info!(job_id = job_id, "暂停任务");

        let mut job = self.repo().get_job_by_id(job_id).await?;

        job.status = JobStatus::Paused;
        job.audit.update_time = Timestamp::now().to_string();

        self.repo().update_job(job).await?;

        info!(job_id = job_id, "任务暂停成功");
        Ok(())
    }

    /// 恢复任务
    pub async fn resume_job(&self, job_id: i64) -> RIE<()> {
        info!(job_id = job_id, "恢复任务");

        let mut job = self.repo().get_job_by_id(job_id).await?;

        job.status = JobStatus::Running;
        job.audit.update_time = Timestamp::now().to_string();

        self.repo().update_job(job).await?;

        info!(job_id = job_id, "任务恢复成功");
        Ok(())
    }

    /// 查询任务列表（分页，id 倒序）
    pub async fn list_jobs(&self, page: i64, page_size: i64) -> RIE<Vec<InfrustJob>> {
        let jobs = self.repo().get_running_jobs(page, page_size).await?;
        Ok(jobs)
    }

    /// 查询任务执行日志（分页，id 倒序）
    pub async fn get_job_logs(&self, job_id: i64, page: Page<InfrustJobLog>) -> RIE<Vec<InfrustJobLog>> {
        let logs = self.repo().get_job_logs(job_id, page).await?;
        Ok(logs)
    }

    /// 验证 Cron 表达式
    fn validate_cron_expression(&self, cron_expression: &str) -> RIE<()> {
        use std::str::FromStr;
        match cron::Schedule::from_str(cron_expression) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("无效的 Cron 表达式: {}", e).into()),
        }
    }

    /// 执行任务
    async fn execute_job(&self, job: &InfrustJob) -> RIE<()> {
        info!(job_id = job.id, job_name = %job.name, "开始执行任务");

        let begin = Timestamp::now();
        let begin_str = begin.to_string();

        let log_id = tx_common::id::next_id() as i64;
        let mut log = InfrustJobLog {
            id: log_id,
            job_id: job.id,
            handler_name: job.handler_name.clone(),
            handler_param: job.handler_param.clone(),
            execute_index: 1,
            begin_time: begin_str.clone(),
            end_time: None,
            duration: None,
            status: ExecutionStatus::Failed,
            result: None,
            audit: AuditFields {
                creator: Some("system".to_string()),
                create_time: begin_str.clone(),
                updater: Some("system".to_string()),
                update_time: begin_str,
            },
            soft_delete: SoftDelete::NORMAL,
        };

        log = self.repo().create_job_log(log).await?;

        // 根据 handler_name 选择执行器
        let executor_type = ExecutorType::from_handler_name(&job.handler_name);
        let result = match executor_type {
            ExecutorType::Internal => {
                self.internal_exec()
                    .execute(job.id, &job.handler_name, job.handler_param.as_deref())
                    .await
            }
            ExecutorType::Shell => {
                self.shell_exec()
                    .execute(job.id, &job.handler_name, job.handler_param.as_deref())
                    .await
            }
            ExecutorType::Python => {
                self.python_exec()
                    .execute(job.id, &job.handler_name, job.handler_param.as_deref())
                    .await
            }
        };

        // 更新执行日志
        let end = Timestamp::now();
        let duration_ms = (end.as_millisecond() - begin.as_millisecond()) as i32;

        log.end_time = Some(end.to_string());
        log.duration = Some(duration_ms);
        log.status = result.status;
        log.result = result.result;
        log.audit.update_time = end.to_string();

        self.repo().update_job_log(log).await?;

        // 检查是否需要重试
        if result.status != ExecutionStatus::Success && job.retry_count > 0 {
            info!(
                job_id = job.id,
                retry_count = job.retry_count,
                "任务执行失败，准备重试"
            );
            // TODO: 实现重试逻辑 — 需要异步重试，可能涉及任务重新调度
            todo!("任务重试逻辑尚未实现");
        }

        info!(
            job_id = job.id,
            status = ?result.status,
            duration_ms = duration_ms,
            "任务执行完成"
        );

        Ok(())
    }

    /// 调度器主循环
    async fn scheduler_loop(&self, token: CancellationToken) -> RIE<()> {
        info!("调度器主循环启动");

        let mut interval = tokio::time::interval(self.config.poll_interval());

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let _now = Timestamp::now();

                    // TODO: 实现 Cron 表达式调度逻辑
                    // 1. 查询所有运行中的任务
                    // 2. 解析 Cron 表达式（使用 jiff::civil::DateTime）
                    // 3. 计算下次执行时间
                    // 4. 如果到达执行时间，执行任务
                    debug!("调度器轮询 (Cron 调度逻辑尚未实现)");
                }
                _ = token.cancelled() => {
                    info!("调度器收到关闭信号，正在停止...");
                    break;
                }
            }
        }

        info!("调度器主循环停止");
        Ok(())
    }
}

impl CompInit for JobPlugin {
    /// 构建时初始化
    fn inner_init(&mut self, _: &InnerContext) -> RIE<()> {
        info!("JobPlugin: 初始化开始");
        info!("JobPlugin: 初始化完成");
        Ok(())
    }

    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            info!("JobPlugin: 异步初始化开始");

            // 获取数据库实例
            let toasty_plugin = ctx.inject::<ToastyPlugin>();

            // 创建数据访问层（内部构造，非 DI 注入）
            let repository = Arc::new(JobRepository::new(toasty_plugin));

            // 创建执行器（内部构造，非 DI 注入）
            let config = ctx.inject::<JobConfig>();
            let internal_executor = Arc::new(InternalJobExecutor::new());
            let shell_executor = Arc::new(ShellJobExecutor::new(config.shell_timeout()));
            let python_executor = Arc::new(PythonJobExecutor::new(
                config.python_path.clone(),
                config.python_timeout(),
            ));

            // 设置到 JobPlugin 的 OnceLock 字段
            let plugin = ctx.inject::<JobPlugin>();
            plugin.repository.set(repository).map_err(|_| {
                anyhow::anyhow!("JobPlugin: repository 已初始化")
            })?;
            plugin.internal_executor.set(internal_executor).map_err(|_| {
                anyhow::anyhow!("JobPlugin: internal_executor 已初始化")
            })?;
            plugin.shell_executor.set(shell_executor).map_err(|_| {
                anyhow::anyhow!("JobPlugin: shell_executor 已初始化")
            })?;
            plugin.python_executor.set(python_executor).map_err(|_| {
                anyhow::anyhow!("JobPlugin: python_executor 已初始化")
            })?;

            info!("JobPlugin: 异步初始化完成");
            Ok(())
        }
    );

    async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            info!("JobPlugin: 启动调度器");

            let plugin = ctx.inject::<JobPlugin>();

            if !plugin.config.enabled {
                info!("JobPlugin: 调度器未启用，跳过启动");
                return Ok(());
            }

            // 启动调度器主循环
            let plugin_clone = plugin.clone();
            let token_clone = token.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = plugin_clone.scheduler_loop(token_clone).await {
                    error!(error = %e, "调度器主循环异常退出");
                }
            });

            if plugin.scheduler_handle.set(handle).is_err() {
                warn!("JobPlugin: 调度器已启动");
            }

            info!("JobPlugin: 调度器启动完成");

            // 等待关闭信号
            token.cancelled().await;
            info!("JobPlugin: 收到关闭信号，正在优雅关闭...");

            Ok(())
        }
    );

    fn init_sort() -> i32 {
        i32::MAX - 10
    }
}
