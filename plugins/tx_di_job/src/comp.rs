use jiff::{Timestamp, civil, tz::TimeZone};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use tx_di_core::{App, Component, DepsTuple, RIE};

use crate::config::JobConfig;
use crate::err::{JobErr, JobResult};
use crate::executors::{
    ExecutorType, InternalJobExecutor, JobExecutor, PythonJobExecutor, ShellJobExecutor,
};
use crate::models::{
    AuditFields, ExecutionStatus, InfrustJob, InfrustJobLog, JobStatus, SoftDelete,
};
use crate::repository::JobRepository;
use tx_common::page::Page;
use tx_di_toasty::ToastyPlugin;

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
#[derive(Component)]
#[component(app_async_init, app_async_run, init_sort = i32::MAX - 10)]
pub struct JobPlugin {
    /// 配置引用（DI 注入）
    pub config: Arc<JobConfig>,

    /// 数据库访问层（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub repository: OnceLock<Arc<JobRepository>>,

    /// 内部函数执行器（内部构造，非 DI 注入）
    #[tx_cst(Arc::new(InternalJobExecutor::new()))]
    pub internal_executor: Arc<InternalJobExecutor>,

    /// Shell 脚本执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub shell_executor: OnceLock<Arc<ShellJobExecutor>>,

    /// Python 脚本执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub python_executor: OnceLock<Arc<PythonJobExecutor>>,
}

impl JobPlugin {
    // ---- 辅助访问器 ----

    fn repo(&self) -> &Arc<JobRepository> {
        self.repository
            .get()
            .expect("JobRepository 未初始化，请确保 async_init 已完成")
    }

    fn internal_exec(&self) -> &Arc<InternalJobExecutor> {
        &self.internal_executor
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

    /// 根据 handler_name 自动识别执行器类型并执行任务（不依赖插件自身的任务表）
    pub async fn execute_by_type(
        &self,
        job_id: i64,
        handler_name: &str,
        handler_param: Option<&str>,
    ) -> JobResult {
        let executor_type = ExecutorType::from_handler_name(&handler_name);
        match executor_type {
            ExecutorType::Internal => {
                self.internal_exec()
                    .execute(job_id, handler_name, handler_param)
                    .await
            }
            ExecutorType::Shell => {
                self.shell_exec()
                    .execute(job_id, handler_name, handler_param)
                    .await
            }
            ExecutorType::Python => {
                self.python_exec()
                    .execute(job_id, handler_name, handler_param)
                    .await
            }
        }
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

        let now = Timestamp::now();
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
                create_time: now,
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

        job.audit.update_time = Timestamp::now();

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
        job.audit.update_time = Timestamp::now();

        self.repo().update_job(job).await?;

        info!(job_id = job_id, "任务暂停成功");
        Ok(())
    }

    /// 恢复任务
    pub async fn resume_job(&self, job_id: i64) -> RIE<()> {
        info!(job_id = job_id, "恢复任务");

        let mut job = self.repo().get_job_by_id(job_id).await?;

        job.status = JobStatus::Running;
        job.audit.update_time = Timestamp::now();

        self.repo().update_job(job).await?;

        info!(job_id = job_id, "任务恢复成功");
        Ok(())
    }

    /// 查询任务列表（分页，id 倒序）
    pub async fn list_jobs(&self, page: Page<InfrustJob>) -> RIE<Vec<InfrustJob>> {
        let jobs = self.repo().get_running_jobs(page).await?;
        Ok(jobs)
    }

    /// 查询任务执行日志（分页，id 倒序）
    pub async fn get_job_logs(
        &self,
        job_id: i64,
        page: Page<InfrustJobLog>,
    ) -> RIE<Vec<InfrustJobLog>> {
        let logs = self.repo().get_job_logs(job_id, page).await?;
        Ok(logs)
    }

    /// 验证 Cron 表达式
    fn validate_cron_expression(&self, cron_expression: &str) -> RIE<()> {
        use std::str::FromStr;
        match cron::Schedule::from_str(cron_expression) {
            Ok(_) => Ok(()),
            Err(e) => Err(tx_error::AppError::with_context(
                JobErr::CronParseFailed,
                format!("无效的 Cron 表达式: {}", e),
            )),
        }
    }

    /// 执行任务（含重试机制）
    ///
    /// 首次执行 + `retry_count` 次重试，每次重试前等待 `retry_interval` 秒。
    /// 每次尝试（含首次和重试）都会独立记录一条执行日志。
    async fn execute_job(&self, job: &InfrustJob) -> RIE<()> {
        info!(job_id = job.id, job_name = %job.name, "开始执行任务");

        let max_attempts = 1 + job.retry_count.max(0) as usize;

        for attempt in 0..max_attempts {
            let execute_index = (attempt + 1) as i16;
            let is_retry = attempt > 0;

            if is_retry {
                info!(
                    job_id = job.id,
                    attempt = attempt + 1,
                    max_attempts = max_attempts,
                    interval_secs = job.retry_interval,
                    "等待后重试任务"
                );
                tokio::time::sleep(Duration::from_secs(job.retry_interval as u64)).await;
            }

            let result = self.execute_single_attempt(job, execute_index).await?;

            if result.status == ExecutionStatus::Success {
                info!(job_id = job.id, attempt = attempt + 1, "任务执行成功");
                return Ok(());
            }

            // 本次尝试失败，如果还有重试次数则继续
            if attempt + 1 < max_attempts {
                warn!(
                    job_id = job.id,
                    attempt = attempt + 1,
                    remaining = max_attempts - attempt - 1,
                    "任务执行失败，准备重试"
                );
            }
        }

        warn!(
            job_id = job.id,
            attempts = max_attempts,
            "任务所有尝试均已失败"
        );
        Ok(())
    }

    /// 执行单次任务尝试（创建日志 → 执行 → 更新日志）
    async fn execute_single_attempt(&self, job: &InfrustJob, execute_index: i16) -> RIE<JobResult> {
        let begin = Timestamp::now();

        let log_id = tx_common::id::next_id() as i64;
        let mut log = InfrustJobLog {
            id: log_id,
            job_id: job.id,
            handler_name: job.handler_name.clone(),
            handler_param: job.handler_param.clone(),
            execute_index,
            begin_time: begin,
            end_time: None,
            duration: None,
            status: ExecutionStatus::Failed,
            result: None,
            audit: AuditFields {
                creator: Some("system".to_string()),
                create_time: begin,
                updater: Some("system".to_string()),
                update_time: begin,
            },
            soft_delete: SoftDelete::NORMAL,
        };

        log = self.repo().create_job_log(log).await?;

        let result = self
            .execute_by_type(job.id, &job.handler_name, job.handler_param.as_deref())
            .await;

        let end = Timestamp::now();
        let duration_ms = (end.as_millisecond() - begin.as_millisecond()) as i32;

        log.end_time = Some(end);
        log.duration = Some(duration_ms);
        log.status = result.status;
        log.result = result.result.clone();
        log.audit.update_time = end;

        self.repo().update_job_log(log).await?;

        debug!(
            job_id = job.id,
            execute_index = execute_index,
            status = ?result.status,
            duration_ms = duration_ms,
            "单次尝试完成"
        );

        Ok(result)
    }

    /// 调度器主循环
    ///
    /// 每个轮询周期检查所有运行中任务，匹配 cron 表达式，到期触发执行。
    /// 通过 `last_trigger` 记录每次触发的精确时间槽，避免同一分钟重复执行。
    async fn scheduler_loop(&self, token: CancellationToken) -> RIE<()> {
        info!("调度器主循环启动");

        let mut interval = tokio::time::interval(self.config.poll_interval());
        // 跟踪每个任务的上次触发时间槽: (年, 月, 日, 时, 分)
        let mut last_trigger: HashMap<i64, (i16, i8, i8, i8, i8)> = HashMap::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let now_ts = Timestamp::now();
                    let now_dt = now_ts.to_zoned(TimeZone::UTC).datetime();

                    // 查询所有运行中的任务
                    let jobs = match self.repo().get_all_running_jobs().await {
                        Ok(jobs) => jobs,
                        Err(e) => {
                            error!(error = %e, "查询运行中任务失败");
                            continue;
                        }
                    };

                    // 清理已移除任务的缓存
                    let active_ids: std::collections::HashSet<i64> =
                        jobs.iter().map(|j| j.id).collect();
                    last_trigger.retain(|id, _| active_ids.contains(id));

                    for job in &jobs {
                        if !cron_matches(&job.cron_expression, &now_dt) {
                            continue;
                        }

                        // 当前时间槽
                        let slot = (
                            now_dt.year(),
                            now_dt.month(),
                            now_dt.day(),
                            now_dt.hour(),
                            now_dt.minute(),
                        );

                        // 检查是否已在当前分钟触发过
                        if last_trigger.get(&job.id) == Some(&slot) {
                            continue;
                        }

                        info!(
                            job_id = job.id,
                            job_name = %job.name,
                            "到达调度时间，触发任务执行"
                        );

                        if let Err(e) = self.execute_job(job).await {
                            error!(
                                job_id = job.id,
                                error = %e,
                                "任务执行失败"
                            );
                        }

                        last_trigger.insert(job.id, slot);
                    }

                    debug!(
                        active_jobs = jobs.len(),
                        "调度器轮询完成"
                    );
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

// ── Cron 表达式匹配（基于 jiff::civil::DateTime） ──────────

/// 判断 cron 表达式（5 字段）是否匹配给定的日期时间
///
/// 支持: `*`(通配)、`N`(指定值)、`N,M`(列表)、`N-M`(范围)、`*/N`(步进)、`N-M/N`(范围步进)
fn cron_matches(expr: &str, dt: &civil::DateTime) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    // cron: min hour dom month dow
    let dow = dt.weekday().to_sunday_zero_offset() as u32; // Sunday=0

    cron_field_match(fields[0], dt.minute() as u32, 0, 59)
        && cron_field_match(fields[1], dt.hour() as u32, 0, 23)
        && cron_field_match(fields[2], dt.day() as u32, 1, 31)
        && cron_field_match(fields[3], dt.month() as u32, 1, 12)
        && cron_field_match(fields[4], dow, 0, 7)
}

/// 匹配单个 cron 字段值
fn cron_field_match(field: &str, value: u32, min: u32, max: u32) -> bool {
    if field == "*" {
        return true;
    }
    for part in field.split(',') {
        if cron_part_match(part.trim(), value, min, max) {
            return true;
        }
    }
    false
}

/// 匹配单个 cron 字段片段（不含逗号分隔）
fn cron_part_match(part: &str, value: u32, min: u32, _max: u32) -> bool {
    // */N 步进
    if let Some(step_str) = part.strip_prefix("*/") {
        if let Ok(step) = step_str.parse::<u32>() {
            return step > 0 && (value - min) % step == 0;
        }
    }
    // N-M/N 范围步进
    if let Some(slash_pos) = part.find('/') {
        let range_part = &part[..slash_pos];
        let step_str = &part[slash_pos + 1..];
        if let (Some(dash_pos), Ok(step)) = (range_part.find('-'), step_str.parse::<u32>()) {
            if let (Ok(start), Ok(end)) = (
                range_part[..dash_pos].parse::<u32>(),
                range_part[dash_pos + 1..].parse::<u32>(),
            ) {
                return step > 0 && value >= start && value <= end && (value - start) % step == 0;
            }
        }
    }
    // N-M 范围
    if let Some(dash_pos) = part.find('-') {
        if let (Ok(start), Ok(end)) = (
            part[..dash_pos].parse::<u32>(),
            part[dash_pos + 1..].parse::<u32>(),
        ) {
            return value >= start && value <= end;
        }
    }
    // 数值
    if let Ok(v) = part.parse::<u32>() {
        return v == value;
    }
    // 名称（月份/星期）
    match part.to_lowercase().as_str() {
        "jan" => value == 1,
        "feb" => value == 2,
        "mar" => value == 3,
        "apr" => value == 4,
        "may" => value == 5,
        "jun" => value == 6,
        "jul" => value == 7,
        "aug" => value == 8,
        "sep" => value == 9,
        "oct" => value == 10,
        "nov" => value == 11,
        "dec" => value == 12,
        "sun" => value == 0,
        "mon" => value == 1,
        "tue" => value == 2,
        "wed" => value == 3,
        "thu" => value == 4,
        "fri" => value == 5,
        "sat" => value == 6,
        _ => false,
    }
}

tx_di_core::async_method!(
    /// `#[component(app_async_init)]` 回调：初始化执行器和 Repository
    fn app_async_init(comp: Arc<JobPlugin>, app: Arc<App>) -> RIE<()> {
        info!("JobPlugin: 异步初始化开始");
        // 获取数据库实例
        let toasty_plugin = app.inject::<ToastyPlugin>();

        // 创建数据访问层（内部构造，非 DI 注入）
        let repository = Arc::new(JobRepository::new(toasty_plugin));

        // 创建执行器（内部构造，非 DI 注入）
        let shell_executor = Arc::new(ShellJobExecutor::new(comp.config.shell_timeout()));
        let python_executor = Arc::new(PythonJobExecutor::new(
            comp.config.python_path.clone(),
            comp.config.python_timeout(),
        ));

        // 设置到 JobPlugin 的 OnceLock 字段
        comp.repository
            .set(repository)
            .map_err(|_| JobErr::RepositoryAlreadyInit)?;
        comp.shell_executor
            .set(shell_executor)
            .map_err(|_| JobErr::ShellExecutorAlreadyInit)?;
        comp.python_executor
            .set(python_executor)
            .map_err(|_| JobErr::PythonExecutorAlreadyInit)?;

        info!("JobPlugin: 异步初始化完成");
        Ok(())
    }
);

tx_di_core::async_method!(
    /// `#[component(app_async_run)]` 回调：启动调度器主循环
    fn app_async_run(comp: Arc<JobPlugin>, _app: Arc<App>, token: CancellationToken) -> RIE<()> {
        if !comp.config.enabled {
            info!("JobPlugin: 调度器未启用，跳过启动");
            return Ok(());
        }

        info!("JobPlugin: 启动调度器");

        // 框架已将 async_run 放入 tokio::spawn，此处直接占用当前任务执行调度循环
        if let Err(e) = comp.scheduler_loop(token).await {
            error!(error = %e, "调度器主循环异常退出");
        }

        info!("JobPlugin: 调度器已停止");
        Ok(())
    }
);
