//! 操作审计日志模型

use toasty::Model;

/// 操作审计日志
///
/// 记录平台上所有重要操作，满足安全审计需求。
#[derive(Debug, Clone, Model)]
#[table = "gb_audit_log"]
pub struct GbAuditLog {
    /// 主键 ID（自增）
    #[key]
    #[auto]
    pub id: u64,

    /// 操作人
    #[default("".to_string())]
    pub operator: String,

    /// 操作类型（login/logout/query/ptz/playback/config 等）
    #[index]
    pub action: String,

    /// 操作目标（设备ID/通道ID/用户ID等）
    #[default("".to_string())]
    pub target: String,

    /// 操作详情（JSON 格式）
    #[default("".to_string())]
    pub detail: String,

    /// 客户端 IP
    #[default("".to_string())]
    pub client_ip: String,

    /// User-Agent
    #[default("".to_string())]
    pub user_agent: String,

    /// 操作结果：success / failure
    #[default("success".to_string())]
    pub result: String,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,
}
