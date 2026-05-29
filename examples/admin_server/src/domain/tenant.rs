//! 租户聚合
//!
//! SaaS 多租户核心，支持租户隔离和套餐管理。

use std::fmt::Display;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toasty::Model;

// ─── 枚举定义 ──────────────────────────────────────────────

/// 租户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum TenantStatus {
    /// 正常
    #[column(variant = 0)]
    Active,
    /// 禁用
    #[column(variant = 1)]
    Disabled,
}

impl Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantStatus::Active => write!(f, "active"),
            TenantStatus::Disabled => write!(f, "disabled"),
        }
    }
}

// ─── 租户套餐 ──────────────────────────────────────────────

/// 租户套餐实体
///
/// 定义租户可用的功能范围（通过菜单 ID 列表控制）。
#[derive(Debug, Clone, Model)]
#[table = "system_tenant_package"]
pub struct TenantPackage {
    /// 套餐 ID
    #[key]
    #[auto]
    pub id: u64,

    /// 套餐名称
    pub name: String,

    /// 套餐状态
    pub status: TenantStatus,

    /// 备注
    pub remark: Option<String>,

    /// 关联的菜单 ID 列表
    pub menu_ids: Vec<u64>,

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

impl TenantPackage {
    /// 创建新套餐
    pub fn new(name: String) -> Self {
        Self {
            id: 0,
            name,
            status: TenantStatus::Active,
            remark: None,
            menu_ids: vec![],
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 设置套餐菜单权限
    pub fn set_menu_ids(&mut self, menu_ids: Vec<u64>) {
        self.menu_ids = menu_ids;
    }

    /// 检查套餐是否包含指定菜单
    pub fn has_menu(&self, menu_id: u64) -> bool {
        self.menu_ids.contains(&menu_id)
    }
}

// ─── 租户聚合根 ──────────────────────────────────────────────

/// 租户实体
///
/// 多租户隔离的核心：同一租户下的用户、角色、权限相互隔离。
#[derive(Debug, Clone, Model)]
#[table = "system_tenant"]
pub struct Tenant {
    /// 租户 ID
    #[key]
    #[auto]
    pub id: u64,

    /// 租户名称
    pub name: String,

    /// 联系人用户 ID
    pub contact_user_id: Option<u64>,

    /// 联系人姓名
    pub contact_name: Option<String>,

    /// 联系电话
    pub contact_mobile: Option<String>,

    /// 租户状态
    pub status: TenantStatus,

    /// 绑定域名列表
    pub websites: Vec<String>,

    /// 关联的套餐 ID
    pub package_id: Option<u64>,

    /// 过期时间（None 表示永不过期）
    pub expire_time: Option<jiff::Timestamp>,

    /// 账号数量上限
    #[default(0i32)]
    pub account_count: i32,

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

impl Tenant {
    /// 创建新租户
    pub fn new(name: String) -> Self {
        Self {
            id: 0,
            name,
            contact_user_id: None,
            contact_name: None,
            contact_mobile: None,
            status: TenantStatus::Active,
            websites: vec![],
            package_id: None,
            expire_time: None,
            account_count: 0,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 租户是否可用（检查状态 + 过期时间）
    pub fn is_active(&self) -> bool {
        if self.status != TenantStatus::Active || self.deleted != 0 {
            return false;
        }
        if let Some(expire) = self.expire_time {
            if jiff::Timestamp::now() > expire {
                return false;
            }
        }
        true
    }

    /// 检查域名是否绑定到此租户
    pub fn matches_domain(&self, domain: &str) -> bool {
        self.websites.iter().any(|w| w == domain)
    }

    /// 绑定域名
    pub fn add_website(&mut self, website: String) {
        if !self.websites.contains(&website) {
            self.websites.push(website);
        }
    }

    /// 解绑域名
    pub fn remove_website(&mut self, website: &str) {
        self.websites.retain(|w| w != website);
    }

    /// 设置套餐
    pub fn set_package(&mut self, package_id: u64) {
        self.package_id = Some(package_id);
    }

    /// 设置过期时间
    pub fn set_expire_time(&mut self, expire_time: jiff::Timestamp) {
        self.expire_time = Some(expire_time);
    }

    /// 软删除
    pub fn mark_deleted(&mut self) {
        self.deleted = 1;
    }
}

// ─── 仓储 trait ──────────────────────────────────────────────

/// 租户仓储 trait
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Tenant>, anyhow::Error>;
    async fn find_by_domain(&self, domain: &str) -> Result<Option<Tenant>, anyhow::Error>;
    async fn find_page(
        &self,
        keyword: Option<&str>,
        status: Option<TenantStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Tenant>, u64), anyhow::Error>;
    async fn save(&self, tenant: &Tenant) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_package_by_id(&self, id: u64) -> Result<Option<TenantPackage>, anyhow::Error>;
    async fn find_all_packages(&self) -> Result<Vec<TenantPackage>, anyhow::Error>;
}
