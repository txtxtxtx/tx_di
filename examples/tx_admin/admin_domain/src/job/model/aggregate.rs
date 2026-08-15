//! 任务域聚合根

use serde::{Deserialize, Serialize};

use crate::AggregateRoot;
use crate::job::model::event::JobEvent;
use crate::job::model::value_object::{ExecutionStatus, JobStatus};
use crate::shared::model::AuditFields;

/// 定时任务聚合根
///
/// 封装一个可调度执行的作业单元及其不变量。
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
#[aggregate_root(event = crate::job::model::event::JobEvent)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: JobStatus,
    pub handler_name: String,
    pub handler_param: Option<String>,
    pub cron_expression: String,
    pub retry_count: i32,
    pub retry_interval: i32,
    pub monitor_timeout: i32,
    pub audit: AuditFields,
    events: Vec<JobEvent>,
}

impl Job {
    /// 创建任务（`events` 初始化为空）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        name: String,
        status: JobStatus,
        handler_name: String,
        handler_param: Option<String>,
        cron_expression: String,
        retry_count: i32,
        retry_interval: i32,
        monitor_timeout: i32,
        audit: AuditFields,
    ) -> Self {
        Self {
            id,
            name,
            status,
            handler_name,
            handler_param,
            cron_expression,
            retry_count,
            retry_interval,
            monitor_timeout,
            audit,
            events: Vec::new(),
        }
    }

    /// 是否已逻辑删除
    pub fn is_deleted(&self) -> bool {
        self.audit.is_deleted()
    }

    /// 是否处于运行中
    pub fn is_running(&self) -> bool {
        self.status == JobStatus::Running
    }

    /// 校验任务不变量
    ///
    /// 返回错误信息（`Option<String>`），`None` 表示校验通过。
    pub fn validate(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("任务名称不能为空".to_string());
        }
        if self.handler_name.trim().is_empty() {
            return Some("处理器名称不能为空".to_string());
        }
        if self.cron_expression.trim().is_empty() {
            return Some("cron 表达式不能为空".to_string());
        }
        if self.retry_count < 0 {
            return Some("重试次数不能为负".to_string());
        }
        if self.retry_interval < 0 {
            return Some("重试间隔不能为负".to_string());
        }
        if self.monitor_timeout < 0 {
            return Some("监控超时不能为负".to_string());
        }
        None
    }
}

/// 任务执行日志聚合根
///
/// 记录单次任务执行的详情（时间、耗时、结果等）。
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
#[aggregate_root(event = crate::job::model::event::JobEvent)]
pub struct JobLog {
    pub id: u64,
    pub job_id: u64,
    pub handler_name: String,
    pub handler_param: Option<String>,
    pub execute_index: i16,
    pub begin_time: jiff::Timestamp,
    pub end_time: Option<jiff::Timestamp>,
    pub duration: Option<i32>,
    pub status: ExecutionStatus,
    pub result: Option<String>,
    pub audit: AuditFields,
    events: Vec<JobEvent>,
}

impl JobLog {
    /// 创建执行日志（`events` 初始化为空）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        job_id: u64,
        handler_name: String,
        handler_param: Option<String>,
        execute_index: i16,
        begin_time: jiff::Timestamp,
        end_time: Option<jiff::Timestamp>,
        duration: Option<i32>,
        status: ExecutionStatus,
        result: Option<String>,
        audit: AuditFields,
    ) -> Self {
        Self {
            id,
            job_id,
            handler_name,
            handler_param,
            execute_index,
            begin_time,
            end_time,
            duration,
            status,
            result,
            audit,
            events: Vec::new(),
        }
    }

    /// 是否已逻辑删除
    pub fn is_deleted(&self) -> bool {
        self.audit.is_deleted()
    }
}
