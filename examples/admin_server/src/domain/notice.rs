//! 通知公告聚合

use serde::{Deserialize, Serialize};
use toasty::Model;

/// 通知类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum NoticeType {
    /// 通知
    #[column(variant = 1)]
    Notice,
    /// 公告
    #[column(variant = 2)]
    Announcement,
}

/// 通知公告实体
#[derive(Debug, Clone, Model)]
#[table = "system_notice"]
pub struct Notice {
    #[key]
    #[auto]
    pub id: u64,
    pub title: String,
    pub content: String,
    pub notice_type: NoticeType,
    /// 状态（0=正常, 1=关闭）
    #[default(0u8)]
    pub status: u8,
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

impl Notice {
    pub fn new(title: String, content: String, notice_type: NoticeType, tenant_id: u64) -> Self {
        Self {
            id: 0, title, content, notice_type, status: 0, tenant_id,
            creator: None, updater: None,
            created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }
}

#[async_trait::async_trait]
pub trait NoticeRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Notice>, anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<Notice>, u64), anyhow::Error>;
    async fn save(&self, notice: &Notice) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
