use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOperateLogCommand {
    pub trace_id: String,
    pub user_id: u64,
    pub user_type: i32,
    pub log_type: String,
    pub sub_type: String,
    pub biz_id: u64,
    pub action: String,
    pub success: i32,
    pub extra: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperateLogQueryRequest {
    pub user_id: Option<u64>,
    pub log_type: Option<String>,
    pub sub_type: Option<String>,
    pub success: Option<i32>,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
    pub page: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OperateLogResponse {
    pub id: u64,
    pub trace_id: String,
    pub user_id: u64,
    pub user_type: i32,
    pub log_type: String,
    pub sub_type: String,
    pub biz_id: u64,
    pub action: String,
    pub success: i32,
    pub extra: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub user_ip: Option<String>,
}

impl From<admin_domain::log::model::aggregate::OperateLog> for OperateLogResponse {
    fn from(log: admin_domain::log::model::aggregate::OperateLog) -> Self {
        Self {
            id: log.id,
            trace_id: log.trace_id,
            user_id: log.user_id,
            user_type: log.user_type,
            log_type: log.log_type,
            sub_type: log.sub_type,
            biz_id: log.biz_id,
            action: log.action,
            success: log.success,
            extra: log.extra,
            request_method: log.request_method,
            request_url: log.request_url,
            user_ip: log.user_ip,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoginLogCommand {
    pub user_id: u64,
    pub user_type: i32,
    pub username: String,
    pub login_ip: String,
    pub login_type: String,
    pub result: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginLogQueryRequest {
    pub user_id: Option<u64>,
    pub username: Option<String>,
    pub login_ip: Option<String>,
    pub login_type: Option<String>,
    pub result: Option<i32>,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
    pub page: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LoginLogResponse {
    pub id: u64,
    pub user_id: u64,
    pub user_type: i32,
    pub username: String,
    pub login_ip: String,
    pub login_type: String,
    pub result: i32,
    pub msg: Option<String>,
}

impl From<admin_domain::log::model::aggregate::LoginLog> for LoginLogResponse {
    fn from(log: admin_domain::log::model::aggregate::LoginLog) -> Self {
        Self {
            id: log.id,
            user_id: log.user_id,
            user_type: log.user_type,
            username: log.username,
            login_ip: log.login_ip,
            login_type: log.login_type,
            result: log.result,
            msg: log.msg,
        }
    }
}
