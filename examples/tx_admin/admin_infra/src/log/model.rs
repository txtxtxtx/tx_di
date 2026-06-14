use toasty::Model;

/// 操作日志表
#[derive(Debug, Clone, Model)]
#[table = "sys_operate_log"]
pub struct SysOperateLog {
    #[key]
    #[auto]
    pub id: i64,

    #[default("".to_string())]
    pub trace_id: String,

    #[default(0)]
    pub user_id: i64,

    #[default(0)]
    pub user_type: i32,

    #[default("".to_string())]
    pub log_type: String,

    #[default("".to_string())]
    pub sub_type: String,

    #[default(0)]
    pub biz_id: i64,

    #[default("".to_string())]
    pub action: String,

    #[default(1)]
    pub success: i32,

    #[default("".to_string())]
    pub extra: String,

    #[default("".to_string())]
    pub request_method: String,

    #[default("".to_string())]
    pub request_url: String,

    #[default("".to_string())]
    pub user_ip: String,

    #[default("".to_string())]
    pub user_agent: String,

    #[default(0)]
    pub tenant_id: i32,

    #[default("".to_string())]
    pub creator: String,

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(0)]
    pub deleted: i32,
}

/// 登录日志表
#[derive(Debug, Clone, Model)]
#[table = "sys_login_log"]
pub struct SysLoginLog {
    #[key]
    #[auto]
    pub id: i64,

    #[default(0)]
    pub user_id: i64,

    #[default(0)]
    pub user_type: i32,

    #[default("".to_string())]
    pub username: String,

    #[default("".to_string())]
    pub login_ip: String,

    #[default("".to_string())]
    pub login_location: String,

    #[default("".to_string())]
    pub browser: String,

    #[default("".to_string())]
    pub os: String,

    #[default("".to_string())]
    pub login_type: String,

    #[default(0)]
    pub result: i32,

    #[default("".to_string())]
    pub msg: String,

    #[default("".to_string())]
    pub login_time: String,

    #[default(0)]
    pub tenant_id: i32,

    #[default("".to_string())]
    pub creator: String,

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(0)]
    pub deleted: i32,
}
