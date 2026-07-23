use tx_error::CodeMsg;
use crate::ExecutionStatus;

/// Job 插件业务错误码
///
/// 使用 `tx_error::CodeMsg` 派生宏自动生成：
/// - `AppErrCode` 转换
/// - `From<JobErr> for AppError`
///
/// 动态上下文通过 `AppError::with_context()` 携带：
/// ```ignore
/// Err(AppError::with_context(JobErr::JobNotFound, format!("id={}", 42)))
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("JOB")]
pub enum JobErr {
    /// 任务不存在
    #[err(1001, "任务不存在")]
    JobNotFound,

    /// 无效的 Cron 表达式
    #[err(1002, "无效的 Cron 表达式")]
    InvalidCronExpression,

    /// 任务执行失败
    #[err(1003, "任务执行失败")]
    ExecutionFailed,

    /// 任务执行超时
    #[err(1004, "任务执行超时")]
    ExecutionTimeout,

    /// 未找到处理器
    #[err(1005, "未找到处理器")]
    HandlerNotFound,

    /// Cron 表达式解析失败
    #[err(1006, "Cron 表达式解析失败")]
    CronParseFailed,

    /// Repository 重复初始化
    #[err(1007, "JobPlugin: repository 已初始化")]
    RepositoryAlreadyInit,

    /// InternalExecutor 重复初始化
    #[err(1008, "JobPlugin: internal_executor 已初始化")]
    InternalExecutorAlreadyInit,

    /// ShellExecutor 重复初始化
    #[err(1009, "JobPlugin: shell_executor 已初始化")]
    ShellExecutorAlreadyInit,

    /// PythonExecutor 重复初始化
    #[err(1010, "JobPlugin: python_executor 已初始化")]
    PythonExecutorAlreadyInit,

    /// Semaphore 重复初始化
    #[err(1011, "JobPlugin: semaphore 已初始化")]
    SemaphoreAlreadyInit,
}

// ── 执行结果 ────────────────────────────────────────────────

/// 任务执行结果（值类型，非错误）
#[derive(Debug, Clone)]
pub struct JobResult {
    pub status: ExecutionStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}


