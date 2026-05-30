//! 岗位聚合

use async_trait::async_trait;
use super::dept::CommonStatus;

#[derive(Debug, Clone)]
pub struct Post {
    pub id: u64, pub code: String, pub name: String, pub sort: i32,
    pub status: CommonStatus, pub remark: Option<String>, pub tenant_id: u64,
    pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}
impl Post {
    pub fn new(tenant_id: u64, code: String, name: String, sort: i32) -> Self { Self { id: 0, code, name, sort, status: CommonStatus::Enable, remark: None, tenant_id, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } }
    pub fn is_active(&self) -> bool { self.status.is_enable() && self.deleted == 0 }
    pub fn mark_deleted(&mut self) { self.deleted = 1; }
}

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Post>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Post>, anyhow::Error>;
    async fn save(&self, post: &Post) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
