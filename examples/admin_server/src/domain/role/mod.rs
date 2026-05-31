//! 角色聚合

use std::fmt::Display;
use async_trait::async_trait;
use crate::domain::data_permission::DataScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum RoleStatus {
    #[column(variant = 0)] Active,
    #[column(variant = 1)] Disabled,
}
impl Display for RoleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { RoleStatus::Active => write!(f, "active"), RoleStatus::Disabled => write!(f, "disabled") }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum RoleType {
    #[column(variant = 0)] Custom,
    #[column(variant = 1)] BuiltIn,
}
impl Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { RoleType::BuiltIn => write!(f, "built_in"), RoleType::Custom => write!(f, "custom") }
    }
}
impl RoleType { pub fn is_built_in(&self) -> bool { matches!(self, RoleType::BuiltIn) } }

#[derive(Debug, Clone)]
pub struct Role {
    pub id: u64, pub tenant_id: u64, pub name: String, pub code: String, pub sort: i32,
    pub data_scope: DataScope, pub data_scope_dept_ids: Vec<u64>,
    pub status: RoleStatus, pub role_type: RoleType, pub remark: Option<String>,
    pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}

impl Role {
    pub fn new(tenant_id: u64, name: String, code: String, sort: i32) -> Self {
        Self {
            id: 0, tenant_id, name, code, sort, data_scope: DataScope::Self_, data_scope_dept_ids: vec![],
            status: RoleStatus::Active, role_type: RoleType::Custom, remark: None,
            creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0,
        }
    }
    pub fn is_active(&self) -> bool { self.status == RoleStatus::Active && self.deleted == 0 }
    pub fn is_built_in(&self) -> bool { self.role_type.is_built_in() }
    pub fn set_data_scope(&mut self, scope: DataScope, dept_ids: Vec<u64>) { self.data_scope = scope; self.data_scope_dept_ids = dept_ids; }
    pub fn disable(&mut self) { self.status = RoleStatus::Disabled; }
    pub fn enable(&mut self) { self.status = RoleStatus::Active; }
    pub fn mark_deleted(&mut self) -> Result<(), &'static str> { if self.is_built_in() { return Err("内置角色不可删除"); } self.deleted = 1; Ok(()) }
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Role>, anyhow::Error>;
    async fn find_by_code(&self, code: &str) -> Result<Option<Role>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Role>, anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Role>, u64), anyhow::Error>;
    async fn save(&self, role: &Role) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_by_ids(&self, ids: &[u64]) -> Result<Vec<Role>, anyhow::Error>;
}
pub mod repo;
