//! 操作日志聚合

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct OperateLog { pub id: u64, pub trace_id: Option<String>, pub user_id: u64, pub user_type: u8, pub op_type: String, pub sub_type: String, pub biz_id: u64, pub action: String, pub success: bool, pub extra: String, pub request_method: Option<String>, pub request_url: Option<String>, pub user_ip: Option<String>, pub user_agent: Option<String>, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
pub trait OperateLogRepository: Send + Sync {
    async fn save(&self, log: &OperateLog) -> Result<(), anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<OperateLog>, u64), anyhow::Error>;
}
pub mod repo;
