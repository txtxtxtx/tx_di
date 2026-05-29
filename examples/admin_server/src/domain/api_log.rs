//! API 日志聚合

use toasty::Model;

/// API 访问日志实体
#[derive(Debug, Clone, Model)]
#[table = "infra_api_access_log"]
pub struct ApiAccessLog {
    #[key]
    #[auto]
    pub id: u64,
    pub trace_id: Option<String>,
    pub user_id: Option<u64>,
    #[default(0u8)]
    pub user_type: u8,
    pub application_name: Option<String>,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub request_params: Option<String>,
    pub response_body: Option<String>,
    pub user_ip: Option<String>,
    pub user_agent: Option<String>,
    pub operate_module: Option<String>,
    pub operate_name: Option<String>,
    pub operate_type: Option<u8>,
    pub begin_time: Option<jiff::Timestamp>,
    pub end_time: Option<jiff::Timestamp>,
    #[default(0i32)]
    pub duration: i32,
    #[default(0i32)]
    pub result_code: i32,
    pub result_msg: Option<String>,
    pub tenant_id: u64,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

/// API 错误日志实体
#[derive(Debug, Clone, Model)]
#[table = "infra_api_error_log"]
pub struct ApiErrorLog {
    #[key]
    #[auto]
    pub id: u64,
    pub trace_id: Option<String>,
    pub user_id: Option<u64>,
    #[default(0u8)]
    pub user_type: u8,
    pub application_name: Option<String>,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub request_params: Option<String>,
    pub user_ip: Option<String>,
    pub user_agent: Option<String>,
    pub exception_time: Option<jiff::Timestamp>,
    pub exception_name: Option<String>,
    pub exception_message: Option<String>,
    pub exception_root_cause_message: Option<String>,
    pub exception_stack_trace: Option<String>,
    pub exception_class_name: Option<String>,
    pub exception_file_name: Option<String>,
    pub exception_method_name: Option<String>,
    pub exception_line_number: Option<i32>,
    #[default(0u8)]
    pub process_status: u8,
    pub process_time: Option<jiff::Timestamp>,
    pub process_user_id: Option<u64>,
    pub tenant_id: u64,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

#[async_trait::async_trait]
pub trait ApiLogRepository: Send + Sync {
    async fn save_access_log(&self, log: &ApiAccessLog) -> Result<(), anyhow::Error>;
    async fn save_error_log(&self, log: &ApiErrorLog) -> Result<(), anyhow::Error>;
    async fn find_access_log_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<ApiAccessLog>, u64), anyhow::Error>;
    async fn find_error_log_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<ApiErrorLog>, u64), anyhow::Error>;
}
