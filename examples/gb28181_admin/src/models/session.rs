//! 会话记录模型

use toasty::Model;

/// GB28181 媒体会话记录
#[derive(Debug, Clone, Model)]
#[table = "gb_session"]
pub struct GbSessionRecord {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub call_id: String,

    #[index]
    pub device_id: String,

    #[index]
    pub channel_id: String,

    #[default(0)]
    pub rtp_port: i32,

    #[default("".to_string())]
    pub ssrc: String,

    #[default("".to_string())]
    pub stream_id: String,

    #[default("realtime".to_string())]
    pub session_type: String,

    #[default(true)]
    pub active: bool,

    /// 开始时间
    #[default(jiff::Timestamp::now())]
    pub started_at: jiff::Timestamp,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,
}
