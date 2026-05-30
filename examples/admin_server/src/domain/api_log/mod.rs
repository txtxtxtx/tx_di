//! API 日志聚合

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ApiAccessLog { pub id: u64, pub trace_id: Option<String>, pub user_id: Option<u64>, pub user_type: u8, pub application_name: Option<String>, pub request_method: Option<String>, pub request_url: Option<String>, pub request_params: Option<String>, pub response_body: Option<String>, pub user_ip: Option<String>, pub user_agent: Option<String>, pub operate_module: Option<String>, pub operate_name: Option<String>, pub operate_type: Option<u8>, pub begin_time: Option<jiff::Timestamp>, pub end_time: Option<jiff::Timestamp>, pub duration: i32, pub result_code: i32, pub result_msg: Option<String>, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[derive(Debug, Clone)]
pub struct ApiErrorLog { pub id: u64, pub trace_id: Option<String>, pub user_id: Option<u64>, pub user_type: u8, pub application_name: Option<String>, pub request_method: Option<String>, pub request_url: Option<String>, pub request_params: Option<String>, pub user_ip: Option<String>, pub user_agent: Option<String>, pub exception_time: Option<jiff::Timestamp>, pub exception_name: Option<String>, pub exception_message: Option<String>, pub exception_root_cause_message: Option<String>, pub exception_stack_trace: Option<String>, pub exception_class_name: Option<String>, pub exception_file_name: Option<String>, pub exception_method_name: Option<String>, pub exception_line_number: Option<i32>, pub process_status: u8, pub process_time: Option<jiff::Timestamp>, pub process_user_id: Option<u64>, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
pub trait ApiLogRepository: Send + Sync {
    async fn save_access_log(&self, log: &ApiAccessLog) -> Result<(), anyhow::Error>;
    async fn save_error_log(&self, log: &ApiErrorLog) -> Result<(), anyhow::Error>;
    async fn find_access_log_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<ApiAccessLog>, u64), anyhow::Error>;
    async fn find_error_log_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<ApiErrorLog>, u64), anyhow::Error>;
}
pub mod repo;
