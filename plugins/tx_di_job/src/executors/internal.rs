use dashmap::DashMap;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use crate::err::JobResult;
use crate::models::ExecutionStatus;
use crate::executors::JobExecutor;

/// 内部函数执行器
///
/// 用于执行 Rust 异步/同步函数，具有超时保护机制。
/// 处理器函数会被放入 `tokio::task::spawn_blocking` 执行，
/// 避免阻塞调度器主循环。
///
/// # 使用示例
///
/// ```rust,ignore
/// let executor = InternalJobExecutor::new(Duration::from_secs(300));
/// executor.register("send_email", |param| {
///     // 发送邮件逻辑
///     JobResult {
///         status: ExecutionStatus::Success,
///         result: Some("邮件发送成功".to_string()),
///         error: None,
///     }
/// });
/// ```
pub struct InternalJobExecutor {
    /// 注册的函数映射表（无锁并发）
    handlers: DashMap<String, Arc<dyn Fn(Option<&str>) -> JobResult + Send + Sync>>,
    /// 执行超时时间
    timeout: Duration,
}

impl InternalJobExecutor {
    /// 创建新的内部函数执行器
    pub fn new(timeout: Duration) -> Self {
        Self {
            handlers: DashMap::new(),
            timeout,
        }
    }

    /// 注册任务处理器
    pub fn register<F>(&self, name: &str, handler: F)
    where
        F: Fn(Option<&str>) -> JobResult + Send + Sync + 'static,
    {
        self.handlers.insert(name.to_string(), Arc::new(handler));
        tracing::info!("已注册内部任务处理器: {}", name);
    }

    /// 注销任务处理器
    pub fn unregister(&self, name: &str) {
        self.handlers.remove(name);
        tracing::info!("已注销内部任务处理器: {}", name);
    }

    /// 检查处理器是否存在
    pub fn has_handler(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }
}

#[async_trait]
impl JobExecutor for InternalJobExecutor {
    async fn execute(&self, job_id: i64, handler_name: &str, param: Option<&str>) -> JobResult {
        tracing::info!(job_id = job_id, handler = handler_name, "执行内部任务");

        let handler_arc = match self.handlers.get(handler_name) {
            Some(h) => h.clone(), // 克隆 Arc，释放锁
            None => {
                tracing::error!(job_id = job_id, handler = handler_name, "未找到处理器");
                return JobResult {
                    status: ExecutionStatus::Failed,
                    result: None,
                    error: Some(format!("未找到处理器: {}", handler_name)),
                };
            }
        };

        // 将参数转为 owned String，跨线程安全
        let owned_param = param.map(|s| s.to_string());

        // 在阻塞线程池中执行，防止同步函数阻塞异步运行时
        // 并受超时保护，防止死循环/长耗时任务卡死调度器
        let result = timeout(self.timeout, tokio::task::spawn_blocking(move || {
            handler_arc(owned_param.as_deref())
        }))
        .await;

        match result {
            // spawn_blocking 正常完成（含超时）
            Ok(Ok(job_result)) => {
                tracing::info!(
                    job_id = job_id,
                    status = ?job_result.status,
                    "内部任务执行完成"
                );
                job_result
            }
            // spawn_blocking 任务本身 panic 或取消
            Ok(Err(join_err)) => {
                tracing::error!(
                    job_id = job_id,
                    error = %join_err,
                    "内部任务执行异常"
                );
                JobResult {
                    status: ExecutionStatus::Failed,
                    result: None,
                    error: Some(format!("内部任务执行异常: {}", join_err)),
                }
            }
            // 超时
            Err(_) => {
                tracing::warn!(job_id = job_id, handler = handler_name, "内部任务执行超时");
                JobResult {
                    status: ExecutionStatus::Timeout,
                    result: None,
                    error: Some("内部任务执行超时".to_string()),
                }
            }
        }
    }
}
