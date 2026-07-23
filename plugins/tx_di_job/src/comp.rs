use jiff::Timestamp;
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
    #[tx_cst(OnceLock::new())]
    pub internal_executor: OnceLock<Arc<InternalJobExecutor>>,

    /// Shell 脚本执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub shell_executor: OnceLock<Arc<ShellJobExecutor>>,

    /// Python 脚本执行器（内部构造，非 DI 注入）
    #[tx_cst(OnceLock::new())]
    pub python_executor: OnceLock<Arc<PythonJobExecutor>>,

    /// 并发执行信号量（限制同时运行的任务数）
    #[tx_cst(OnceLock::new())]
    pub semaphore: OnceLock<Arc<tokio::sync::Semaphore>>,
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
    ///
    /// 支持格式（与 `cron` crate 一致）：
    /// - 7 字段: `sec min hour dom mon dow year`
    /// - 6 字段: `sec min hour dom mon dow`  
    /// - 5 字段: `min hour dom mon dow`（向后兼容旧数据，内部自动转换）
    fn validate_cron_expression(&self, cron_expression: &str) -> RIE<()> {
        match parse_cron_schedule(cron_expression) {
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
    /// 使用 `Semaphore` 控制并发执行数，超过 `thread_pool_size` 的任务延迟到下一轮。
    async fn scheduler_loop(self: Arc<Self>, token: CancellationToken) -> RIE<()> {
        info!("调度器主循环启动");

        let semaphore = self
            .semaphore
            .get()
            .expect("Semaphore 未初始化，请确保 async_init 已完成");

        let mut interval = tokio::time::interval(self.config.poll_interval());
        // 跟踪每个任务的上次触发时间槽: (年, 月, 日, 时, 分)
        let mut last_trigger: HashMap<i64, (i16, i8, i8, i8, i8)> = HashMap::new();
        // 缓存已解析的 Cron Schedule，避免每轮循环重复解析
        let mut cached_schedules: HashMap<i64, cron::Schedule> = HashMap::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let now_chrono = chrono::Utc::now();
                    let now_dt = Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC).datetime();

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
                    cached_schedules.retain(|id, _| active_ids.contains(id));

                    for job in &jobs {
                        // 解析并缓存 Schedule（支持 5/6/7 字段，表达式已在 create/update 时验证过）
                        let schedule = cached_schedules.entry(job.id).or_insert_with(|| {
                            parse_cron_schedule(&job.cron_expression)
                                .expect("Cron 表达式应在 create/update 时已验证过")
                        });

                        if !schedule.includes(now_chrono) {
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

                        // 获取并发执行许可（非阻塞）
                        let permit = match semaphore.clone().try_acquire_owned() {
                            Ok(p) => p,
                            Err(_) => {
                                warn!(
                                    job_id = job.id,
                                    thread_pool_size = self.config.thread_pool_size,
                                    "执行器池已满，跳过本次调度，任务将在下一轮尝试"
                                );
                                continue;
                            }
                        };

                        info!(
                            job_id = job.id,
                            job_name = %job.name,
                            "到达调度时间，分派任务执行"
                        );

                        // 分派到独立任务中执行，避免阻塞调度循环
                        let job_id = job.id;
                        let this = self.clone();
                        let job = job.clone();
                        tokio::spawn(async move {
                            let _permit = permit; // 保持许可，释放后其他任务可执行
                            if let Err(e) = this.execute_job(&job).await {
                                error!(
                                    job_id = job.id,
                                    error = %e,
                                    "任务执行失败"
                                );
                            }
                        });

                        last_trigger.insert(job_id, slot);
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

// ── Cron 表达式解析 ─────────────────────────────────────

/// 解析 Cron 表达式，支持 5/6/7 字段格式
///
/// | 字段数 | 格式 | 说明 |
/// |--------|------|------|
/// | 7 | `sec min hour dom mon dow year` | 标准格式 |
/// | 6 | `sec min hour dom mon dow` | 年字段默认 `*` |
/// | 5 | `min hour dom mon dow` | 向后兼容旧数据，自动转为 `0 min hour dom mon dow *` |
fn parse_cron_schedule(expr: &str) -> Result<cron::Schedule, cron::error::Error> {
    // 直接解析 6/7 字段
    if let Ok(s) = expr.parse::<cron::Schedule>() {
        return Ok(s);
    }
    // 5 字段向后兼容：sec=0, year=*
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() == 5 {
        format!("0 {} *", expr).parse()
    } else {
        // 仍失败则返回原始错误
        expr.parse()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::executors::ExecutorType;

    // ── parse_cron_schedule ──────────────────────────────

    #[test]
    fn test_parse_7_fields() {
        let s = parse_cron_schedule("0 30 9 * * Mon-Fri 2026").unwrap();
        assert!(s.includes(chrono::Utc.with_ymd_and_hms(2026, 7, 23, 9, 30, 0).unwrap()));
    }

    #[test]
    fn test_parse_6_fields() {
        let s = parse_cron_schedule("0 0 2 * * *").unwrap();
        assert!(s.includes(chrono::Utc.with_ymd_and_hms(2026, 7, 23, 2, 0, 0).unwrap()));
    }

    #[test]
    fn test_parse_5_fields_backward_compat() {
        let s = parse_cron_schedule("0 2 * * *").unwrap();
        assert!(s.includes(chrono::Utc.with_ymd_and_hms(2026, 7, 23, 2, 0, 0).unwrap()));
    }

    #[test]
    fn test_parse_5_fields_is_equivalent_to_7() {
        // 5 字段 "0 2 * * *"（min hour dom mon dow）应等价于 7 字段 "0 0 2 * * * *"
        let s5 = parse_cron_schedule("0 2 * * *").unwrap();
        let s7 = parse_cron_schedule("0 0 2 * * * *").unwrap();
        let ts = chrono::Utc.with_ymd_and_hms(2026, 7, 23, 2, 0, 0).unwrap();
        assert_eq!(s5.includes(ts), s7.includes(ts));
    }

    #[test]
    fn test_parse_invalid_expression() {
        assert!(parse_cron_schedule("invalid cron").is_err());
        assert!(parse_cron_schedule("").is_err());
    }

    #[test]
    fn test_parse_complex_expression() {
        // 复杂表达式："每天 9:30,12:30,15:30 的工作日"
        let s = parse_cron_schedule("0 30 9,12,15 * * Mon-Fri").unwrap();
        let ts = chrono::Utc.with_ymd_and_hms(2026, 7, 23, 9, 30, 0).unwrap();
        assert!(s.includes(ts));
        // 非工作日的相同时间应不匹配
        let ts_sat = chrono::Utc.with_ymd_and_hms(2026, 7, 25, 9, 30, 0).unwrap();
        assert!(!s.includes(ts_sat));
    }

    // ── ExecutorType ─────────────────────────────────────

    #[test]
    fn test_executor_type_internal() {
        assert_eq!(ExecutorType::from_handler_name("my_func"), ExecutorType::Internal);
        assert_eq!(ExecutorType::from_handler_name("cleanup_logs"), ExecutorType::Internal);
    }

    #[test]
    fn test_executor_type_shell() {
        assert_eq!(ExecutorType::from_handler_name("/opt/scripts/backup.sh"), ExecutorType::Shell);
        assert_eq!(ExecutorType::from_handler_name("test.sh"), ExecutorType::Shell);
    }

    #[test]
    fn test_executor_type_python() {
        assert_eq!(ExecutorType::from_handler_name("/opt/scripts/analyze.py"), ExecutorType::Python);
        assert_eq!(ExecutorType::from_handler_name("script.py"), ExecutorType::Python);
    }

    // ── InfrustJob 软删除 ────────────────────────────────

    #[test]
    fn test_job_is_deleted() {
        let now = jiff::Timestamp::now();
        let job = InfrustJob {
            id: 1,
            name: "test".into(),
            status: JobStatus::Running,
            handler_name: "test".into(),
            handler_param: None,
            cron_expression: "0 2 * * *".into(),
            retry_count: 0,
            retry_interval: 0,
            monitor_timeout: 0,
            audit: AuditFields {
                creator: None,
                create_time: now,
                updater: None,
                update_time: now,
            },
            soft_delete: SoftDelete::NORMAL,
        };
        assert!(!job.is_deleted());
        let deleted = InfrustJob { soft_delete: SoftDelete::DELETED, ..job.clone() };
        assert!(deleted.is_deleted());
    }

    // ── JobConfig ────────────────────────────────────────

    #[test]
    fn test_job_config_defaults() {
        let cfg = JobConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.poll_interval_secs, 1);
        assert_eq!(cfg.shell_timeout_secs, 300);
        assert_eq!(cfg.python_timeout_secs, 300);
        assert_eq!(cfg.thread_pool_size, 4);
        assert_eq!(cfg.internal_timeout_secs, 300);
        assert_eq!(cfg.python_path.to_str().unwrap(), "/usr/bin/python3");
        assert_eq!(cfg.shell_timeout(), Duration::from_secs(300));
        assert_eq!(cfg.python_timeout(), Duration::from_secs(300));
        assert_eq!(cfg.internal_timeout(), Duration::from_secs(300));
    }

    // ── JobResult ────────────────────────────────────────

    #[test]
    fn test_job_result_construction() {
        let r = JobResult {
            status: ExecutionStatus::Success,
            result: Some("ok".into()),
            error: None,
        };
        assert_eq!(r.status, ExecutionStatus::Success);
        assert_eq!(r.result.as_deref(), Some("ok"));
        assert!(r.error.is_none());
    }

    // ── ExecutionStatus ─────────────────────────────────

    #[test]
    fn test_execution_status_variants() {
        assert_eq!(ExecutionStatus::Failed as i32, 0);
        assert_eq!(ExecutionStatus::Success as i32, 1);
        assert_eq!(ExecutionStatus::Timeout as i32, 2);
        assert_eq!(ExecutionStatus::Retrying as i32, 3);
    }

    // ── InternalJobExecutor 超时 ─────────────────────────

    #[tokio::test]
    async fn test_internal_executor_timeout() {
        let executor = InternalJobExecutor::new(Duration::from_millis(10));
        executor.register("blocking_fn", |_| {
            // 模拟死循环/阻塞操作
            std::thread::sleep(Duration::from_secs(5));
            JobResult {
                status: ExecutionStatus::Success,
                result: None,
                error: None,
            }
        });
        let result = executor.execute(1, "blocking_fn", None).await;
        assert_eq!(result.status, ExecutionStatus::Timeout);
        assert_eq!(result.error.as_deref(), Some("内部任务执行超时"));
    }

    #[tokio::test]
    async fn test_internal_executor_success() {
        let executor = InternalJobExecutor::new(Duration::from_secs(30));
        executor.register("success_fn", |_| {
            JobResult {
                status: ExecutionStatus::Success,
                result: Some("done".into()),
                error: None,
            }
        });
        let result = executor.execute(1, "success_fn", None).await;
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.result.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn test_internal_executor_handler_not_found() {
        let executor = InternalJobExecutor::new(Duration::from_secs(30));
        let result = executor.execute(1, "nonexistent", None).await;
        assert_eq!(result.status, ExecutionStatus::Failed);
        assert!(result.error.unwrap().contains("未找到处理器"));
    }

    #[tokio::test]
    async fn test_internal_executor_param_passing() {
        let executor = InternalJobExecutor::new(Duration::from_secs(30));
        executor.register("param_fn", |param| {
            JobResult {
                status: ExecutionStatus::Success,
                result: param.map(|s| s.to_string()),
                error: None,
            }
        });
        let result = executor.execute(1, "param_fn", Some(r#"{"key":"value"}"#)).await;
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.result.as_deref(), Some(r#"{"key":"value"}"#));
    }

    #[tokio::test]
    async fn test_internal_executor_without_timeout_succeeds() {
        let executor = InternalJobExecutor::new(Duration::from_secs(30));
        executor.register("quick_fn", |_| {
            std::thread::sleep(Duration::from_millis(50));
            JobResult {
                status: ExecutionStatus::Success,
                result: Some("quick".into()),
                error: None,
            }
        });
        let result = executor.execute(1, "quick_fn", None).await;
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    // ── Semaphore 并发控制 ──────────────────────────────

    #[tokio::test]
    async fn test_semaphore_limits_concurrency() {
        let sem = tokio::sync::Semaphore::new(2);
        // 初始应该有 2 个许可可用
        let p1 = sem.try_acquire().unwrap();
        let p2 = sem.try_acquire().unwrap();
        assert!(sem.try_acquire().is_err());
        drop(p1);
        drop(p2);
        // 释放后应可再次获取
        let _p = sem.try_acquire().unwrap();
    }
}
async fn app_async_init(comp: Arc<JobPlugin>, app: Arc<App>) -> RIE<()> {
    info!("JobPlugin: 异步初始化开始");
    // 获取数据库实例
    let toasty_plugin = app.inject::<ToastyPlugin>();

    // 创建数据访问层（内部构造，非 DI 注入）
    let repository = Arc::new(JobRepository::new(toasty_plugin));

    // 创建执行器（内部构造，非 DI 注入）
    let internal_executor = Arc::new(InternalJobExecutor::new(comp.config.internal_timeout()));
    let shell_executor = Arc::new(ShellJobExecutor::new(comp.config.shell_timeout()));
    let python_executor = Arc::new(PythonJobExecutor::new(
        comp.config.python_path.clone(),
        comp.config.python_timeout(),
    ));

    // 创建并发控制信号量
    let semaphore = Arc::new(tokio::sync::Semaphore::new(comp.config.thread_pool_size));

    // 设置到 JobPlugin 的 OnceLock 字段
    comp.repository
        .set(repository)
        .map_err(|_| JobErr::RepositoryAlreadyInit)?;
    comp.internal_executor
        .set(internal_executor)
        .map_err(|_| JobErr::InternalExecutorAlreadyInit)?;
    comp.shell_executor
        .set(shell_executor)
        .map_err(|_| JobErr::ShellExecutorAlreadyInit)?;
    comp.python_executor
        .set(python_executor)
        .map_err(|_| JobErr::PythonExecutorAlreadyInit)?;
    comp.semaphore
        .set(semaphore)
        .map_err(|_| JobErr::SemaphoreAlreadyInit)?;

    info!("JobPlugin: 异步初始化完成");
    Ok(())
}

/// `#[component(app_async_run)]` 回调：启动调度器主循环
async fn app_async_run(comp: Arc<JobPlugin>, _app: Arc<App>, token: CancellationToken) -> RIE<()> {
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
