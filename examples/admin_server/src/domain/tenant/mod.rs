//! 租户聚合

use std::fmt::Display;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum TenantStatus {
    #[column(variant = 0)] Active,
    #[column(variant = 1)] Disabled,
}
impl Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { TenantStatus::Active => write!(f, "active"), TenantStatus::Disabled => write!(f, "disabled") }
    }
}

#[derive(Debug, Clone)]
pub struct TenantPackage {
    pub id: u64, pub name: String, pub status: TenantStatus, pub remark: Option<String>,
    pub menu_ids: Vec<u64>, pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}
impl TenantPackage {
    pub fn new(name: String) -> Self { Self { id: 0, name, status: TenantStatus::Active, remark: None, menu_ids: vec![], creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } }
}

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: u64, pub name: String, pub contact_user_id: Option<u64>,
    pub contact_name: Option<String>, pub contact_mobile: Option<String>,
    pub status: TenantStatus, pub websites: Vec<String>, pub package_id: Option<u64>,
    pub expire_time: Option<jiff::Timestamp>, pub account_count: i32,
    pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}
impl Tenant {
    pub fn new(name: String) -> Self { Self { id: 0, name, contact_user_id: None, contact_name: None, contact_mobile: None, status: TenantStatus::Active, websites: vec![], package_id: None, expire_time: None, account_count: 0, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } }
    pub fn is_active(&self) -> bool { self.status == TenantStatus::Active && self.deleted == 0 }
    pub fn mark_deleted(&mut self) { self.deleted = 1; }
}

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Tenant>, anyhow::Error>;
    async fn find_page(&self, keyword: Option<&str>, status: Option<TenantStatus>, page: u64, page_size: u64) -> Result<(Vec<Tenant>, u64), anyhow::Error>;
    async fn save(&self, tenant: &Tenant) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_all_packages(&self) -> Result<Vec<TenantPackage>, anyhow::Error>;
}
pub mod repo;
