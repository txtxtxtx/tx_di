//! 报警记录模型

use toasty::Model;

/// GB28181 报警记录
#[derive(Debug, Clone, Model)]
#[table = "gb_alarm"]
pub struct GbAlarmRecord {
    #[key]
    #[auto]
    pub id: i64,

    #[index]
    pub device_id: String,

    #[index]
    pub channel_id: String,

    #[default(1)]
    pub alarm_method: i32,

    #[default("".to_string())]
    pub alarm_type: String,

    #[default(1)]
    pub alarm_level: i32,

    #[default("".to_string())]
    pub description: String,

    /// 报警时间
    #[default(jiff::Timestamp::now())]
    pub alarm_time: jiff::Timestamp,

    /// 处理状态：0-未处理 1-已确认 2-已处理
    #[default(0)]
    #[index]
    pub status: i32,

    #[default("".to_string())]
    pub handler: String,

    #[default("".to_string())]
    pub handle_remark: String,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,
}
