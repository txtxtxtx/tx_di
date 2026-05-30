//! API 日志仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{ApiAccessLog, ApiErrorLog, ApiLogRepository};

#[derive(Debug, Clone, Model)]
#[table = "infra_api_access_log"]
pub struct ApiAccessLogModel {
    #[key] #[auto] pub id: u64, #[default("".to_string())] pub trace_id: String, #[default(0i64)] #[index] pub user_id: i64,
    #[default(0u8)] pub user_type: u8, #[default("".to_string())] pub application_name: String,
    #[default("".to_string())] pub request_method: String, #[default("".to_string())] pub request_url: String, #[default("".to_string())] pub request_params: String,
    #[default("".to_string())] pub response_body: String, #[default("".to_string())] pub user_ip: String, #[default("".to_string())] pub user_agent: String,
    #[default("".to_string())] pub operate_module: String, #[default("".to_string())] pub operate_name: String, #[default(0u8)] pub operate_type: u8,
    #[default(0i32)] pub duration: i32, #[default(0i32)] pub result_code: i32, #[default("".to_string())] pub result_msg: String,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "infra_api_error_log"]
pub struct ApiErrorLogModel {
    #[key] #[auto] pub id: u64, #[default("".to_string())] pub trace_id: String, #[default(0i64)] #[index] pub user_id: i64,
    #[default(0u8)] pub user_type: u8, #[default("".to_string())] pub application_name: String,
    #[default("".to_string())] pub request_method: String, #[default("".to_string())] pub request_url: String, #[default("".to_string())] pub request_params: String,
    #[default("".to_string())] pub user_ip: String, #[default("".to_string())] pub user_agent: String,
    #[default("".to_string())] pub exception_name: String, #[default("".to_string())] pub exception_message: String,
    #[default("".to_string())] pub exception_root_cause_message: String, #[default("".to_string())] pub exception_stack_trace: String,
    #[default("".to_string())] pub exception_class_name: String, #[default("".to_string())] pub exception_file_name: String,
    #[default("".to_string())] pub exception_method_name: String, #[default(0i32)] pub exception_line_number: i32,
    #[default(0u8)] pub process_status: u8, #[default(0i64)] pub process_user_id: i64,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<ApiAccessLogModel> for ApiAccessLog { fn from(m: ApiAccessLogModel) -> Self { Self { id: m.id, trace_id: if m.trace_id.is_empty() { None } else { Some(m.trace_id) }, user_id: if m.user_id == 0 { None } else { Some(m.user_id as u64) }, user_type: m.user_type, application_name: if m.application_name.is_empty() { None } else { Some(m.application_name) }, request_method: if m.request_method.is_empty() { None } else { Some(m.request_method) }, request_url: if m.request_url.is_empty() { None } else { Some(m.request_url) }, request_params: if m.request_params.is_empty() { None } else { Some(m.request_params) }, response_body: if m.response_body.is_empty() { None } else { Some(m.response_body) }, user_ip: if m.user_ip.is_empty() { None } else { Some(m.user_ip) }, user_agent: if m.user_agent.is_empty() { None } else { Some(m.user_agent) }, operate_module: if m.operate_module.is_empty() { None } else { Some(m.operate_module) }, operate_name: if m.operate_name.is_empty() { None } else { Some(m.operate_name) }, operate_type: if m.operate_type == 0 { None } else { Some(m.operate_type) }, begin_time: None, end_time: None, duration: m.duration, result_code: m.result_code, result_msg: if m.result_msg.is_empty() { None } else { Some(m.result_msg) }, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<ApiErrorLogModel> for ApiErrorLog { fn from(m: ApiErrorLogModel) -> Self { Self { id: m.id, trace_id: if m.trace_id.is_empty() { None } else { Some(m.trace_id) }, user_id: if m.user_id == 0 { None } else { Some(m.user_id as u64) }, user_type: m.user_type, application_name: if m.application_name.is_empty() { None } else { Some(m.application_name) }, request_method: if m.request_method.is_empty() { None } else { Some(m.request_method) }, request_url: if m.request_url.is_empty() { None } else { Some(m.request_url) }, request_params: if m.request_params.is_empty() { None } else { Some(m.request_params) }, user_ip: if m.user_ip.is_empty() { None } else { Some(m.user_ip) }, user_agent: if m.user_agent.is_empty() { None } else { Some(m.user_agent) }, exception_time: None, exception_name: if m.exception_name.is_empty() { None } else { Some(m.exception_name) }, exception_message: if m.exception_message.is_empty() { None } else { Some(m.exception_message) }, exception_root_cause_message: if m.exception_root_cause_message.is_empty() { None } else { Some(m.exception_root_cause_message) }, exception_stack_trace: if m.exception_stack_trace.is_empty() { None } else { Some(m.exception_stack_trace) }, exception_class_name: if m.exception_class_name.is_empty() { None } else { Some(m.exception_class_name) }, exception_file_name: if m.exception_file_name.is_empty() { None } else { Some(m.exception_file_name) }, exception_method_name: if m.exception_method_name.is_empty() { None } else { Some(m.exception_method_name) }, exception_line_number: if m.exception_line_number == 0 { None } else { Some(m.exception_line_number) }, process_status: m.process_status, process_time: None, process_user_id: if m.process_user_id == 0 { None } else { Some(m.process_user_id as u64) }, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyApiLogRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl ApiLogRepository for ToastyApiLogRepository {
    async fn save_access_log(&self, log: &ApiAccessLog) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); toasty::create!(ApiAccessLogModel { trace_id: log.trace_id.clone().unwrap_or_default(), user_id: log.user_id.map(|v| v as i64).unwrap_or_default(), user_type: log.user_type, application_name: log.application_name.clone().unwrap_or_default(), request_method: log.request_method.clone().unwrap_or_default(), request_url: log.request_url.clone().unwrap_or_default(), request_params: log.request_params.clone().unwrap_or_default(), response_body: log.response_body.clone().unwrap_or_default(), user_ip: log.user_ip.clone().unwrap_or_default(), user_agent: log.user_agent.clone().unwrap_or_default(), operate_module: log.operate_module.clone().unwrap_or_default(), operate_name: log.operate_name.clone().unwrap_or_default(), operate_type: log.operate_type.unwrap_or_default(), duration: log.duration, result_code: log.result_code, result_msg: log.result_msg.clone().unwrap_or_default(), tenant_id: log.tenant_id as i64, creator: log.creator.clone().unwrap_or_default(), updater: log.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(()) }
    async fn save_error_log(&self, log: &ApiErrorLog) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); toasty::create!(ApiErrorLogModel { trace_id: log.trace_id.clone().unwrap_or_default(), user_id: log.user_id.map(|v| v as i64).unwrap_or_default(), user_type: log.user_type, application_name: log.application_name.clone().unwrap_or_default(), request_method: log.request_method.clone().unwrap_or_default(), request_url: log.request_url.clone().unwrap_or_default(), request_params: log.request_params.clone().unwrap_or_default(), user_ip: log.user_ip.clone().unwrap_or_default(), user_agent: log.user_agent.clone().unwrap_or_default(), exception_name: log.exception_name.clone().unwrap_or_default(), exception_message: log.exception_message.clone().unwrap_or_default(), exception_root_cause_message: log.exception_root_cause_message.clone().unwrap_or_default(), exception_stack_trace: log.exception_stack_trace.clone().unwrap_or_default(), exception_class_name: log.exception_class_name.clone().unwrap_or_default(), exception_file_name: log.exception_file_name.clone().unwrap_or_default(), exception_method_name: log.exception_method_name.clone().unwrap_or_default(), exception_line_number: log.exception_line_number.unwrap_or_default(), process_status: log.process_status, process_user_id: log.process_user_id.map(|v| v as i64).unwrap_or_default(), tenant_id: log.tenant_id as i64, creator: log.creator.clone().unwrap_or_default(), updater: log.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(()) }
    async fn find_access_log_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<ApiAccessLog>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = ApiAccessLogModel::filter_by_tenant_id(tenant_id as i64).count().exec(&mut db).await? as u64; let models = ApiAccessLogModel::filter_by_tenant_id(tenant_id as i64).offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(ApiAccessLog::from).collect(), total)) }
    async fn find_error_log_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<ApiErrorLog>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = ApiErrorLogModel::filter_by_tenant_id(tenant_id as i64).count().exec(&mut db).await? as u64; let models = ApiErrorLogModel::filter_by_tenant_id(tenant_id as i64).offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(ApiErrorLog::from).collect(), total)) }
}
