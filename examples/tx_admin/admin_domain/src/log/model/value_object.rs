use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperateLogQuery {
    pub user_id: Option<u64>,
    pub log_type: Option<String>,
    pub sub_type: Option<String>,
    pub success: Option<i32>,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoginLogQuery {
    pub user_id: Option<u64>,
    pub username: Option<String>,
    pub login_ip: Option<String>,
    pub login_type: Option<String>,
    pub result: Option<i32>,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
}
