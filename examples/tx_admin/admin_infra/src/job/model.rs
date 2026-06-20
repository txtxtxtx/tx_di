use toasty::Model;

use crate::common::Deleted;

/// 系统定时任务表
#[derive(Debug, Clone, Model)]
#[table = "sys_job"]
pub struct SysJob {
    #[key]
    #[auto]
    pub id: i64,

    pub name: String,

    #[default(1)]
    pub status: i32,

    pub handler_name: String,

    #[default("".to_string())]
    pub handler_param: String,

    pub cron_expression: String,

    #[default(0)]
    pub retry_count: i32,

    #[default(0)]
    pub retry_interval: i32,

    #[default(0)]
    pub monitor_timeout: i32,

    #[default("".to_string())]
    pub creator: String,

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}

/// 系统任务执行日志表
#[derive(Debug, Clone, Model)]
#[table = "sys_job_log"]
pub struct SysJobLog {
    #[key]
    #[auto]
    pub id: i64,

    #[index]
    pub job_id: i64,

    pub handler_name: String,

    #[default("".to_string())]
    pub handler_param: String,

    #[default(1)]
    pub execute_index: i32,

    pub begin_time: String,

    #[default("".to_string())]
    pub end_time: String,

    #[default(0)]
    pub duration: i32,

    #[default(0)]
    pub status: i32,

    #[default("".to_string())]
    pub result: String,

    #[default("".to_string())]
    pub creator: String,

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}
