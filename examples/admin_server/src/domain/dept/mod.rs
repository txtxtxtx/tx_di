//! 部门聚合

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonStatus { Enable, Disable }
impl CommonStatus { pub fn is_enable(&self) -> bool { matches!(self, CommonStatus::Enable) } }
impl std::fmt::Display for CommonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { CommonStatus::Enable => write!(f, "enable"), CommonStatus::Disable => write!(f, "disable") } }
}

#[derive(Debug, Clone)]
pub struct Dept {
    pub id: u64, pub tenant_id: u64, pub name: String, pub parent_id: u64, pub sort: i32,
    pub leader_user_id: Option<u64>, pub phone: Option<String>, pub email: Option<String>,
    pub status: CommonStatus, pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}
impl Dept {
    pub fn new(tenant_id: u64, name: String, parent_id: u64) -> Self { Self { id: 0, tenant_id, name, parent_id, sort: 0, leader_user_id: None, phone: None, email: None, status: CommonStatus::Enable, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } }
    pub fn is_root(&self) -> bool { self.parent_id == 0 }
    pub fn is_active(&self) -> bool { self.status.is_enable() && self.deleted == 0 }
    pub fn mark_deleted(&mut self) { self.deleted = 1; }
}

#[async_trait]
pub trait DeptRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Dept>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Dept>, anyhow::Error>;
    async fn save(&self, dept: &Dept) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
