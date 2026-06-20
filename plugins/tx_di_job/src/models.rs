use serde::{Deserialize, Serialize};
use toasty::{Model, Embed};

/// 审计字段（创建者、创建时间、更新者、更新时间）
#[derive(Debug, Clone, Serialize, Deserialize, Embed)]
pub struct AuditFields {
    pub creator: Option<String>,
    pub create_time: String,
    pub updater: Option<String>,
    pub update_time: String,
}
// ── 任务状态 ────────────────────────────────────────────────

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Embed)]
pub enum JobStatus {
    /// 暂停
    Paused = 0,
    /// 运行中
    Running = 1,
}
/// 软删除字段
///
/// 直接映射到数据库的 `i32` 列（0=正常, 1=已删除），toasty 原生支持枚举映射。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Embed)]
pub enum SoftDelete {
    NORMAL = 0,
    DELETED = 1,
}

impl SoftDelete {
    /// 是否处于正常（未删除）状态
    pub fn is_normal(&self) -> bool {
        matches!(self, SoftDelete::NORMAL)
    }
}
// ── 执行状态 ────────────────────────────────────────────────

/// 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Embed)]
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


/// 定时任务表
/// 定时任务实体，表示一个可调度执行的作业单元。
///
/// 每个任务包含执行处理器信息、调度策略（cron 表达式）、重试配置以及审计与软删除字段。
///
/// # 字段说明
///
/// - `id`：任务唯一标识（主键）。
/// - `name`：任务名称，用于展示与日志标识。
/// - `status`：任务状态（`JobStatus` 枚举，toasty 自动映射为 `i32`）。
/// - `handler_name`：处理器名称，对应已注册的执行器逻辑。
/// - `handler_param`：处理器参数，以 JSON 字符串形式传递，可为空。
/// - `cron_expression`：cron 调度表达式，定义任务的触发周期。
/// - `retry_count`：失败重试次数上限。
/// - `retry_interval`：重试间隔（秒）。
/// - `monitor_timeout`：执行超时监控阈值（秒）。
/// - `audit`：审计字段，记录创建与更新信息。
/// - `soft_delete`：软删除标记，支持逻辑删除恢复。
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct InfrustJob {
    #[key]
    pub id: i64,
    pub name: String,
    #[index]
    pub status: JobStatus,
    pub handler_name: String,
    pub handler_param: Option<String>,
    pub cron_expression: String,
    pub retry_count: i32,
    pub retry_interval: i32,
    pub monitor_timeout: i32,
    pub audit: AuditFields,
    pub soft_delete: SoftDelete,
}


impl InfrustJob {
    pub fn is_deleted(&self) -> bool {
        self.soft_delete == SoftDelete::DELETED
    }
}
/// 定时任务日志表
/// 任务执行日志实体
///
/// 记录每次任务执行的详细信息，包括执行时间、耗时、结果等。
///
/// # 字段说明
///
/// - `id`: 日志唯一标识
/// - `job_id`: 关联的任务 ID
/// - `handler_name`: 执行器名称（如 `internal`、`shell`、`python`）
/// - `handler_param`: 执行器参数（JSON 字符串）
/// - `execute_index`: 执行序号，标识第几次执行（从 1 开始）
/// - `begin_time`: 执行开始时间（RFC3339 格式字符串）
/// - `end_time`: 执行结束时间（RFC3339 格式字符串，执行中为 `None`）
/// - `duration`: 执行耗时（毫秒）
/// - `status`: 执行状态（`ExecutionStatus` 枚举，toasty 自动映射为 `i32`）
/// - `result`: 执行结果或错误信息
/// - `audit`: 审计字段（创建时间、更新时间等）
/// - `soft_delete`: 软删除标记
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct InfrustJobLog {
    #[key]
    pub id: i64,
    #[index]
    pub job_id: i64,
    pub handler_name: String,
    pub handler_param: Option<String>,
    pub execute_index: i16,
    pub begin_time: String,
    pub end_time: Option<String>,
    pub duration: Option<i32>,
    pub status: ExecutionStatus,
    pub result: Option<String>,
    pub audit: AuditFields,
    pub soft_delete: SoftDelete,
}

/// 注册模型到 Toasty
pub fn register_models() -> toasty::ModelSet {
    toasty::models!(InfrustJob, InfrustJobLog)
}
