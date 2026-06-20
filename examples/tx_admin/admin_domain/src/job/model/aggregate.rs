use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent};
use crate::shared::model::value_object::DeletedStatus;
use crate::AggregateRoot;

/// 定时任务聚合根
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
pub struct Job {
    /// 任务ID
    pub id: u64,
    /// 任务名称
    pub name: String,
    /// 状态（0=暂停, 1=运行）
    pub status: i32,
    /// 处理器名称
    pub handler_name: String,
    /// 处理器参数
    pub handler_param: Option<String>,
    /// Cron表达式
    pub cron_expression: String,
    /// 重试次数
    pub retry_count: i32,
    /// 重试间隔（秒）
    pub retry_interval: i32,
    /// 执行超时时间（秒）
    pub monitor_timeout: i32,
    /// 审计字段
    pub audit: AuditFields,
    // 领域事件列表
    events: Vec<DomainEvent>,
}

impl Job {
    /// 从持久化层恢复定时任务（不触发领域事件）
    pub fn restore(
        id: u64,
        name: String,
        status: i32,
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

    /// 创建新定时任务
    pub fn create(
        id: u64,
        name: String,
        handler_name: String,
        handler_param: Option<String>,
        cron_expression: String,
        retry_count: i32,
        retry_interval: i32,
        monitor_timeout: i32,
        creator: Option<String>,
    ) -> Self {
        let mut job = Self {
            id,
            name,
            status: 1,
            handler_name,
            handler_param,
            cron_expression,
            retry_count,
            retry_interval,
            monitor_timeout,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            events: Vec::new(),
        };
        job.add_event(DomainEvent::JobCreated { job_id: id });
        job
    }

    /// 更新任务信息
    pub fn update_info(
        &mut self,
        name: String,
        handler_name: String,
        handler_param: Option<String>,
        cron_expression: String,
        retry_count: i32,
        retry_interval: i32,
        monitor_timeout: i32,
        updater: Option<String>,
    ) {
        self.name = name;
        self.handler_name = handler_name;
        self.handler_param = handler_param;
        self.cron_expression = cron_expression;
        self.retry_count = retry_count;
        self.retry_interval = retry_interval;
        self.monitor_timeout = monitor_timeout;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::JobUpdated { job_id: self.id });
    }

    /// 变更任务状态
    pub fn change_status(&mut self, status: i32, updater: Option<String>) {
        self.status = status;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::JobStatusChanged { job_id: self.id, status });
    }

    /// 软删除
    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.deleted = DeletedStatus::Deleted;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::JobDeleted { job_id: self.id });
    }
}

/// 定时任务执行日志聚合根
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
pub struct JobLog {
    /// 日志ID
    pub id: u64,
    /// 任务ID
    pub job_id: u64,
    /// 处理器名称
    pub handler_name: String,
    /// 处理器参数
    pub handler_param: Option<String>,
    /// 执行序号
    pub execute_index: i32,
    /// 开始时间（ISO 8601）
    pub begin_time: String,
    /// 结束时间（ISO 8601）
    pub end_time: Option<String>,
    /// 执行耗时（毫秒）
    pub duration: Option<i32>,
    /// 执行状态（0=执行中, 1=成功, 2=失败）
    pub status: i32,
    /// 执行结果
    pub result: Option<String>,
    /// 审计字段
    pub audit: AuditFields,
    // 领域事件列表
    events: Vec<DomainEvent>,
}

impl JobLog {
    /// 从持久化层恢复任务日志（不触发领域事件）
    pub fn restore(
        id: u64,
        job_id: u64,
        handler_name: String,
        handler_param: Option<String>,
        execute_index: i32,
        begin_time: String,
        end_time: Option<String>,
        duration: Option<i32>,
        status: i32,
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

    /// 创建新的任务执行日志
    pub fn create(
        id: u64,
        job_id: u64,
        handler_name: String,
        handler_param: Option<String>,
        execute_index: i32,
        creator: Option<String>,
    ) -> Self {
        let mut log = Self {
            id,
            job_id,
            handler_name,
            handler_param,
            execute_index,
            begin_time: Timestamp::now().to_string(),
            end_time: None,
            duration: None,
            status: 0,
            result: None,
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            events: Vec::new(),
        };
        log.add_event(DomainEvent::JobLogCreated { log_id: id });
        log
    }

    /// 执行成功
    pub fn finish_success(&mut self, result: String, updater: Option<String>) {
        let now = Timestamp::now();
        self.end_time = Some(now.to_string());
        self.duration = Some(self.calc_duration(now));
        self.status = 1;
        self.result = Some(result);
        self.audit.updater = updater;
        self.audit.update_time = now;
        self.add_event(DomainEvent::JobLogFinished { log_id: self.id });
    }

    /// 执行失败
    pub fn finish_failure(&mut self, result: String, updater: Option<String>) {
        let now = Timestamp::now();
        self.end_time = Some(now.to_string());
        self.duration = Some(self.calc_duration(now));
        self.status = 2;
        self.result = Some(result);
        self.audit.updater = updater;
        self.audit.update_time = now;
        self.add_event(DomainEvent::JobLogFinished { log_id: self.id });
    }

    /// 计算从 begin_time 到给定时间的毫秒数
    fn calc_duration(&self, end: Timestamp) -> i32 {
        if let Ok(begin) = self.begin_time.parse::<Timestamp>() {
            let span = end - begin;
            span.total(jiff::Unit::Millisecond).unwrap_or(0.0) as i32
        } else {
            0
        }
    }
}
