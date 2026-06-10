//! 日志 DTO

use serde::Serialize;

// ── 登录日志 ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LoginLogDto {
    pub id: u64,
    pub log_type: String,
    pub trace_id: Option<String>,
    pub user_id: Option<u64>,
    pub user_type: u8,
    pub username: Option<String>,
    pub result: String,
    pub user_ip: Option<String>,
    pub user_agent: Option<String>,
    pub tenant_id: u64,
    pub created_at: String,
}

impl From<&crate::domain::login_log::LoginLog> for LoginLogDto {
    fn from(l: &crate::domain::login_log::LoginLog) -> Self {
        Self {
            id: l.id,
            log_type: l.log_type.to_string(),
            trace_id: l.trace_id.clone(),
            user_id: l.user_id,
            user_type: l.user_type,
            username: l.username.clone(),
            result: l.result.to_string(),
            user_ip: l.user_ip.clone(),
            user_agent: l.user_agent.clone(),
            tenant_id: l.tenant_id,
            created_at: l.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

// ── 操作日志 ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OperateLogDto {
    pub id: u64,
    pub trace_id: Option<String>,
    pub user_id: u64,
    pub user_type: u8,
    pub op_type: String,
    pub sub_type: String,
    pub biz_id: u64,
    pub action: String,
    pub success: bool,
    pub extra: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub user_ip: Option<String>,
    pub user_agent: Option<String>,
    pub tenant_id: u64,
    pub creator: Option<String>,
    pub created_at: String,
}

impl From<&crate::domain::operate_log::OperateLog> for OperateLogDto {
    fn from(l: &crate::domain::operate_log::OperateLog) -> Self {
        Self {
            id: l.id,
            trace_id: l.trace_id.clone(),
            user_id: l.user_id,
            user_type: l.user_type,
            op_type: l.op_type.clone(),
            sub_type: l.sub_type.clone(),
            biz_id: l.biz_id,
            action: l.action.clone(),
            success: l.success,
            extra: l.extra.clone(),
            request_method: l.request_method.clone(),
            request_url: l.request_url.clone(),
            user_ip: l.user_ip.clone(),
            user_agent: l.user_agent.clone(),
            tenant_id: l.tenant_id,
            creator: l.creator.clone(),
            created_at: l.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}
