//! 任务域值对象

use serde::{Deserialize, Serialize};

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// 暂停
    Paused = 0,
    /// 运行中
    Running = 1,
}

impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Running
    }
}

/// 任务执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// 失败
    Failed = 0,
    /// 成功
    Success = 1,
    /// 超时
    Timeout = 2,
    /// 重试中
    Retrying = 3,
}

/// 任务查询条件
#[derive(Debug, Clone, Default)]
pub struct JobQuery {
    /// 任务名称（模糊匹配）
    pub name: Option<String>,
    /// 任务状态（0=暂停，1=运行）
    pub status: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}

/// 任务日志查询条件
#[derive(Debug, Clone, Default)]
pub struct JobLogQuery {
    pub job_id: Option<u64>,
    /// 执行状态（0=失败，1=成功）
    pub status: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}
