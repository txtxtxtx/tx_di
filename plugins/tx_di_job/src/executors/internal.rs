use dashmap::DashMap;
use async_trait::async_trait;
use crate::err::JobResult;
use crate::models::ExecutionStatus;
use crate::executors::JobExecutor;

/// 内部函数执行器
///
/// 用于执行 Rust 异步函数
///
/// # 使用示例
///
/// ```rust,ignore
/// let executor = InternalJobExecutor::new();
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
    handlers: DashMap<String, Box<dyn Fn(Option<&str>) -> JobResult + Send + Sync>>,
}

impl InternalJobExecutor {
    /// 创建新的内部函数执行器
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    /// 注册任务处理器
    pub fn register<F>(&self, name: &str, handler: F)
    where
        F: Fn(Option<&str>) -> JobResult + Send + Sync + 'static,
    {
        self.handlers.insert(name.to_string(), Box::new(handler));
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

        match self.handlers.get(handler_name) {
            Some(handler) => {
                let result = handler(param);
                tracing::info!(
                    job_id = job_id,
                    status = ?result.status,
                    "内部任务执行完成"
                );
                result
            }
            None => {
                tracing::error!(job_id = job_id, handler = handler_name, "未找到处理器");
                JobResult {
                    status: ExecutionStatus::Failed,
                    result: None,
                    error: Some(format!("未找到处理器: {}", handler_name)),
                }
            }
        }
    }
}
