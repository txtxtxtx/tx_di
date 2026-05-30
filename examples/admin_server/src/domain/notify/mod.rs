//! 站内信聚合

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct NotifyTemplate { pub id: u64, pub name: String, pub code: String, pub nickname: String, pub content: String, pub template_type: u16, pub params: Option<String>, pub status: u8, pub remark: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[derive(Debug, Clone)]
pub struct NotifyMessage { pub id: u64, pub user_id: u64, pub user_type: u8, pub template_id: u64, pub template_code: String, pub template_nickname: String, pub template_content: String, pub template_type: u16, pub template_params: String, pub read_status: bool, pub read_time: Option<jiff::Timestamp>, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }
impl NotifyMessage { pub fn mark_read(&mut self) { self.read_status = true; self.read_time = Some(jiff::Timestamp::now()); } pub fn is_read(&self) -> bool { self.read_status } }

#[async_trait]
pub trait NotifyRepository: Send + Sync {
    async fn find_template_by_id(&self, id: u64) -> Result<Option<NotifyTemplate>, anyhow::Error>;
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<NotifyTemplate>, u64), anyhow::Error>;
    async fn save_template(&self, tpl: &NotifyTemplate) -> Result<(), anyhow::Error>;
    async fn delete_template(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn save_message(&self, msg: &NotifyMessage) -> Result<(), anyhow::Error>;
    async fn find_message_page(&self, user_id: u64, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<NotifyMessage>, u64), anyhow::Error>;
    async fn count_unread(&self, user_id: u64, tenant_id: u64) -> Result<u64, anyhow::Error>;
    async fn mark_read(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn mark_all_read(&self, user_id: u64, tenant_id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
