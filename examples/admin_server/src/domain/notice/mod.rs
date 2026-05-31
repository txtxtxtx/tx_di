//! 通知公告聚合

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum NoticeType {
    #[column(variant = 0)] Notice,
    #[column(variant = 1)] Announcement,
}
impl std::fmt::Display for NoticeType { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { NoticeType::Notice => write!(f, "notice"), NoticeType::Announcement => write!(f, "announcement") } } }

#[derive(Debug, Clone)]
pub struct Notice { pub id: u64, pub title: String, pub content: String, pub notice_type: NoticeType, pub status: u8, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }
impl Notice { pub fn new(title: String, content: String, notice_type: NoticeType, tenant_id: u64) -> Self { Self { id: 0, title, content, notice_type, status: 0, tenant_id, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } } }

#[async_trait]
pub trait NoticeRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Notice>, anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<Notice>, u64), anyhow::Error>;
    async fn save(&self, notice: &Notice) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
