use std::path::Path;
use std::time::Duration;
use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn, error};
use crate::err::JobResult;
use crate::models::ExecutionStatus;
use crate::executors::JobExecutor;

/// Python 脚本执行器
///
/// 用于执行 Python 脚本
///
/// # 使用示例
///
/// ```rust,ignore
/// let executor = PythonJobExecutor::new("/usr/bin/python3".into(), Duration::from_secs(300));
/// let result = executor.execute(1, "/opt/scripts/analyze.py", Some(r#"{"date": "2024-01-01"}"#)).await;
/// ```
pub struct PythonJobExecutor {
    /// Python 解释器路径
    python_path: std::path::PathBuf,
    /// 执行超时时间
    timeout: Duration,
}

impl PythonJobExecutor {
    /// 创建新的 Python 脚本执行器
    pub fn new(python_path: std::path::PathBuf, timeout: Duration) -> Self {
        Self { python_path, timeout }
    }
    
    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// 设置 Python 解释器路径
    pub fn with_python_path(mut self, python_path: std::path::PathBuf) -> Self {
        self.python_path = python_path;
        self
    }
}

#[async_trait]
impl JobExecutor for PythonJobExecutor {
    async fn execute(&self, job_id: i64, handler_name: &str, param: Option<&str>) -> JobResult {
        info!(job_id = job_id, script = handler_name, "执行 Python 脚本");
        
        // 检查脚本是否存在
        if !Path::new(handler_name).exists() {
            error!(job_id = job_id, script = handler_name, "脚本文件不存在");
            return JobResult {
                status: ExecutionStatus::Failed,
                result: None,
                error: Some(format!("脚本文件不存在: {}", handler_name)),
            };
        }
        
        // 构建命令
        let mut cmd = Command::new(&self.python_path);
        cmd.arg(handler_name);
        if let Some(p) = param {
            cmd.arg(p);
        }
        
        // 设置超时并执行
        let output = match timeout(self.timeout, cmd.output()).await {
            Ok(result) => {
                match result {
                    Ok(output) => output,
                    Err(e) => {
                        error!(job_id = job_id, error = %e, "执行脚本失败");
                        return JobResult {
                            status: ExecutionStatus::Failed,
                            result: None,
                            error: Some(format!("执行脚本失败: {}", e)),
                        };
                    }
                }
            }
            Err(_) => {
                warn!(job_id = job_id, "脚本执行超时");
                return JobResult {
                    status: ExecutionStatus::Timeout,
                    result: None,
                    error: Some("执行超时".to_string()),
                };
            }
        };
        
        // 检查执行结果
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            info!(job_id = job_id, "Python 脚本执行成功");
            JobResult {
                status: ExecutionStatus::Success,
                result: Some(stdout),
                error: None,
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!(job_id = job_id, error = %stderr, "Python 脚本执行失败");
            JobResult {
                status: ExecutionStatus::Failed,
                result: None,
                error: Some(stderr),
            }
        }
    }
}
