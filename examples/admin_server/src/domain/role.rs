//! 角色聚合
//!
//! 角色是权限的集合体，关联菜单权限和数据权限范围。

use std::fmt::Display;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toasty::Model;

use super::data_permission::DataScope;

// ─── 枚举定义 ──────────────────────────────────────────────

/// 角色状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum RoleStatus {
    /// 正常
    #[column(variant = 0)]
    Active,
    /// 禁用
    #[column(variant = 1)]
    Disabled,
}

impl Display for RoleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleStatus::Active => write!(f, "active"),
            RoleStatus::Disabled => write!(f, "disabled"),
        }
    }
}

/// 角色类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum RoleType {
    /// 系统内置角色（不可删除）
    #[column(variant = 1)]
    BuiltIn,
    /// 自定义角色
    #[column(variant = 2)]
    Custom,
}

impl Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleType::BuiltIn => write!(f, "built_in"),
            RoleType::Custom => write!(f, "custom"),
        }
    }
}

impl RoleType {
    pub fn is_built_in(&self) -> bool {
        matches!(self, RoleType::BuiltIn)
    }
}

// ─── 角色聚合根 ──────────────────────────────────────────────

/// 角色实体
///
/// 职责：
/// - 定义角色基本信息
/// - 管理角色与菜单的关联（通过 RoleMenu 关联表）
/// - 管理数据权限范围
/// - 区分内置角色和自定义角色
#[derive(Debug, Clone, Model)]
#[table = "system_role"]
pub struct Role {
    /// 角色 ID
    #[key]
    #[auto]
    pub id: u64,

    /// 所属租户 ID
    pub tenant_id: u64,

    /// 角色名称
    pub name: String,

    /// 角色编码（唯一标识，如 admin、user）
    pub code: String,

    /// 角色排序（越小越靠前）
    #[default(0i32)]
    pub sort: i32,

    /// 数据权限范围（整数判别值）
    pub data_scope: DataScope,

    /// 自定义数据权限部门 ID 列表
    pub data_scope_dept_ids: Vec<u64>,

    /// 角色状态
    pub status: RoleStatus,

    /// 角色类型
    pub role_type: RoleType,

    /// 备注
    pub remark: Option<String>,

    /// 创建者
    pub creator: Option<String>,

    /// 更新者
    pub updater: Option<String>,

    /// 创建时间
    #[auto]
    pub created_at: jiff::Timestamp,

    /// 更新时间
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    /// 软删除标记
    #[default(0u8)]
    pub deleted: u8,
}

// ─── 领域行为 ──────────────────────────────────────────────

impl Role {
    /// 创建新角色
    pub fn new(tenant_id: u64, name: String, code: String, sort: i32) -> Self {
        Self {
            id: 0,
            tenant_id,
            name,
            code,
            sort,
            data_scope: DataScope::Self_,
            data_scope_dept_ids: vec![],
            status: RoleStatus::Active,
            role_type: RoleType::Custom,
            remark: None,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 创建系统内置角色
    pub fn new_built_in(tenant_id: u64, name: String, code: String, sort: i32) -> Self {
        let mut role = Self::new(tenant_id, name, code, sort);
        role.role_type = RoleType::BuiltIn;
        role
    }

    /// 是否可用
    pub fn is_active(&self) -> bool {
        self.status == RoleStatus::Active && self.deleted == 0
    }

    /// 是否为系统内置角色
    pub fn is_built_in(&self) -> bool {
        self.role_type.is_built_in()
    }

    /// 设置数据权限范围
    pub fn set_data_scope(&mut self, scope: DataScope, dept_ids: Vec<u64>) {
        self.data_scope = scope;
        self.data_scope_dept_ids = dept_ids;
    }

    /// 禁用角色
    pub fn disable(&mut self) {
        self.status = RoleStatus::Disabled;
    }

    /// 启用角色
    pub fn enable(&mut self) {
        self.status = RoleStatus::Active;
    }

    /// 更新基本信息
    pub fn update_info(&mut self, name: Option<String>, sort: Option<i32>, remark: Option<String>) {
        if let Some(n) = name {
            self.name = n;
        }
        if let Some(s) = sort {
            self.sort = s;
        }
        if let Some(r) = remark {
            self.remark = Some(r);
        }
    }

    /// 软删除（内置角色不可删除）
    pub fn mark_deleted(&mut self) -> Result<(), &'static str> {
        if self.is_built_in() {
            return Err("内置角色不可删除");
        }
        self.deleted = 1;
        Ok(())
    }
}

// ─── 仓储 trait ──────────────────────────────────────────────

/// 角色仓储 trait
#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Role>, anyhow::Error>;
    async fn find_by_code(&self, code: &str) -> Result<Option<Role>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Role>, anyhow::Error>;
    async fn find_page(
        &self,
        tenant_id: u64,
        keyword: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Role>, u64), anyhow::Error>;
    async fn save(&self, role: &Role) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_by_ids(&self, ids: &[u64]) -> Result<Vec<Role>, anyhow::Error>;
}
