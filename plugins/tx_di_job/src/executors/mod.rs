use async_trait::async_trait;
use crate::err::JobResult;

// 执行器模块
pub mod internal;
pub mod shell;
pub mod python;

// 重新导出
pub use internal::InternalJobExecutor;
pub use shell::ShellJobExecutor;
pub use python::PythonJobExecutor;

/// 任务执行器 trait
#[async_trait]
pub trait JobExecutor: Send + Sync {
    /// 执行任务
    async fn execute(&self, job_id: u64, handler_name: &str, param: Option<&str>) -> JobResult;
}

/// 任务执行器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorType {
    Internal,  // 内部函数
    Shell,      // Shell 脚本
    Python,     // Python 脚本
}

impl ExecutorType {
    /// 根据 handler_name 判断执行器类型
    pub fn from_handler_name(handler_name: &str) -> Self {
        // 路径格式，判断是 Shell 还是 Python 还是内部
        if handler_name.ends_with(".py") {
            ExecutorType::Python
        } else if handler_name.ends_with(".sh") {
            ExecutorType::Shell
        } else {
            // 函数名称格式
            ExecutorType::Internal
        }
    }
}
