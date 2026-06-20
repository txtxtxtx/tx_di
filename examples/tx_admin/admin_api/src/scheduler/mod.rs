//! 定时任务调度引擎
//!
//! 基于 `tokio-cron-scheduler` 实现异步 cron 调度。
//! 启动时从数据库加载所有 status=1 的任务并注册到调度器。

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use admin_domain::job::repository::{JobLogRepository, JobRepository};
use tx_common::id;
use tx_di_core::{tx_comp, App, CancellationToken, CompInit, RIE};
use tx_di_toasty::ToastyConfig;

/// 任务处理器函数类型
type HandlerFn = fn(&str) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;

/// 全局处理器注册表
static HANDLERS: OnceLock<HashMap<String, HandlerFn>> = OnceLock::new();

fn get_handlers() -> &'static HashMap<String, HandlerFn> {
    HANDLERS.get_or_init(|| {
        let mut m: HashMap<String, HandlerFn> = HashMap::new();
        m.insert("noop".to_string(), |_param| {
            Box::pin(async { Ok("ok".to_string()) })
        });
        m.insert("echo".to_string(), |param| {
            let p = param.to_string();
            Box::pin(async move { Ok(p) })
        });
        m
    })
}

/// 调度引擎插件（DI 组件）
///
/// 启动时自动加载活跃任务并注册到 cron 调度器。
#[tx_comp(init)]
pub struct SchedulerPlugin {
    job_repo: Arc<dyn JobRepository>,
    log_repo: Arc<dyn JobLogRepository>,
}

impl CompInit for SchedulerPlugin {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {

            let plugin = ctx.inject::<SchedulerPlugin>();
            let job_repo = plugin.job_repo.clone();
            let log_repo = plugin.log_repo.clone();

            tokio::spawn(async move {
                // 等待数据库初始化完成（DbInitPlugin 需要时间建表和种子数据）
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                if let Err(e) = run_scheduler(job_repo, log_repo).await {
                    tracing::error!(error = %e, "定时任务调度引擎启动失败");
                }
            });

            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 在 AdminPlugin (MAX-100) 之前，但异步任务会延迟启动
        i32::MAX - 150
    }
}

/// 启动调度引擎主循环
async fn run_scheduler(
    job_repo: Arc<dyn JobRepository>,
    log_repo: Arc<dyn JobLogRepository>,
) -> Result<(), anyhow::Error> {
    use tokio_cron_scheduler::JobScheduler as CronScheduler;

    let scheduler = CronScheduler::new().await?;

    // 加载活跃任务（表可能不存在，优雅处理）
    let active_jobs = match job_repo.find_active().await {
        Ok(jobs) => jobs,
        Err(e) => {
            tracing::warn!(error = %e, "加载定时任务失败（表可能不存在），跳过调度");
            return Ok(());
        }
    };
    tracing::info!(count = active_jobs.len(), "加载活跃定时任务");

    for job in active_jobs {
        match make_cron_job(&job, log_repo.clone()) {
            Ok(cron_job) => {
                if let Err(e) = scheduler.add(cron_job).await {
                    tracing::warn!(job_id = job.id, error = %e, "注册定时任务失败");
                } else {
                    tracing::info!(job_id = job.id, name = %job.name, "定时任务已注册");
                }
            }
            Err(e) => {
                tracing::warn!(job_id = job.id, error = %e, "创建定时任务失败");
            }
        }
    }

    scheduler.start().await?;
    tracing::info!("定时任务调度引擎已启动");

    // 保持运行直到进程退出
    tokio::signal::ctrl_c().await.ok();
    Ok(())
}

/// 从领域聚合创建 cron 任务
fn make_cron_job(
    job: &admin_domain::job::model::aggregate::Job,
    log_repo: Arc<dyn JobLogRepository>,
) -> Result<tokio_cron_scheduler::Job, anyhow::Error> {
    let job_id = job.id;
    let handler_name = job.handler_name.clone();
    let handler_param = job.handler_param.clone().unwrap_or_default();
    let retry_count = job.retry_count;
    let retry_interval = job.retry_interval;

    let cron_job = tokio_cron_scheduler::Job::new(
        job.cron_expression.as_str(),
        move |_uuid, _lock| {
            let handler_name = handler_name.clone();
            let handler_param = handler_param.clone();
            let log_repo = log_repo.clone();

            tokio::spawn(async move {
                do_execute(job_id, &handler_name, &handler_param, retry_count, retry_interval, log_repo).await;
            });
        },
    )?;

    Ok(cron_job)
}

/// 执行单个任务（含重试）
async fn do_execute(
    job_id: u64,
    handler_name: &str,
    handler_param: &str,
    retry_count: i32,
    retry_interval: i32,
    log_repo: Arc<dyn JobLogRepository>,
) {
    tracing::info!(job_id, handler = %handler_name, "开始执行定时任务");

    // 创建执行日志
    let log_id = id::next_id();
    let log_agg = admin_domain::job::model::aggregate::JobLog::create(
        log_id,
        job_id,
        handler_name.to_string(),
        Some(handler_param.to_string()),
        1,
        Some("scheduler".to_string()),
    );

    if let Err(e) = log_repo.insert(&log_agg).await {
        tracing::error!(job_id, error = %e, "创建任务日志失败");
        return;
    }

    // 查找处理器
    let handler = match get_handlers().get(handler_name) {
        Some(h) => h,
        None => {
            let msg = format!("未找到处理器: {}", handler_name);
            tracing::error!(job_id, handler = %handler_name, msg);
            let mut log = log_agg;
            log.finish_failure(msg, Some("scheduler".to_string()));
            let _ = log_repo.update(&log).await;
            return;
        }
    };

    // 执行（带重试）
    let mut last_result = Err("未执行".to_string());
    for attempt in 0..=(retry_count as u32) {
        if attempt > 0 {
            tracing::info!(job_id, attempt, "重试定时任务");
            tokio::time::sleep(tokio::time::Duration::from_secs(retry_interval as u64)).await;
        }

        match handler(handler_param).await {
            Ok(r) => {
                last_result = Ok(r);
                break;
            }
            Err(e) => {
                tracing::warn!(job_id, attempt, error = %e, "定时任务执行失败");
                last_result = Err(e);
            }
        }
    }

    // 更新日志
    let mut log = log_agg;
    match last_result {
        Ok(r) => {
            tracing::info!(job_id, result = %r, "定时任务执行成功");
            log.finish_success(r, Some("scheduler".to_string()));
        }
        Err(e) => {
            tracing::error!(job_id, error = %e, "定时任务最终执行失败");
            log.finish_failure(e, Some("scheduler".to_string()));
        }
    }

    if let Err(e) = log_repo.update(&log).await {
        tracing::warn!(error = %e, "更新任务日志失败");
    }
}
