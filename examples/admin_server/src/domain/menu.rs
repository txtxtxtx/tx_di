//! 菜单聚合
//!
//! 支持目录、菜单、按钮三种类型，采用层级结构。
//! 菜单是全局的，没有 tenant_id。

use std::fmt::Display;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toasty::Model;

// ─── 枚举定义 ──────────────────────────────────────────────

/// 菜单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum MenuType {
    /// 目录（仅作为分组）
    #[column(variant = 1)]
    Directory,
    /// 菜单（页面入口）
    #[column(variant = 2)]
    Menu,
    /// 按钮（页面内操作）
    #[column(variant = 3)]
    Button,
}

impl Display for MenuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuType::Directory => write!(f, "directory"),
            MenuType::Menu => write!(f, "menu"),
            MenuType::Button => write!(f, "button"),
        }
    }
}

/// 菜单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum MenuStatus {
    /// 正常
    #[column(variant = 0)]
    Active,
    /// 停用
    #[column(variant = 1)]
    Disabled,
}

// ─── 菜单实体 ──────────────────────────────────────────────

/// 菜单实体
///
/// 职责：
/// - 定义系统菜单树结构
/// - 管理页面路由、组件、图标
/// - 定义按钮权限标识
/// - 控制菜单可见性和缓存策略
#[derive(Debug, Clone, Model)]
#[table = "system_menu"]
pub struct Menu {
    /// 菜单 ID
    #[key]
    #[auto]
    pub id: u64,

    /// 菜单名称
    pub name: String,

    /// 权限标识（如 system:user:create）
    pub permission: Option<String>,

    /// 菜单类型
    pub menu_type: MenuType,

    /// 显示顺序
    #[default(0i32)]
    pub sort: i32,

    /// 父菜单 ID（0 表示顶级菜单）
    #[default(0u64)]
    pub parent_id: u64,

    /// 路由路径
    #[column("path")]
    pub route_path: Option<String>,

    /// 菜单图标
    pub icon: Option<String>,

    /// 组件路径
    pub component: Option<String>,

    /// 组件名称
    pub component_name: Option<String>,

    /// 菜单状态
    pub status: MenuStatus,

    /// 是否可见
    #[default(true)]
    pub visible: bool,

    /// 是否缓存（KeepAlive）
    #[default(false)]
    pub keep_alive: bool,

    /// 是否始终显示
    #[default(false)]
    pub always_show: bool,

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

impl Menu {
    /// 创建目录
    pub fn directory(name: String, parent_id: u64, sort: i32, route_path: Option<String>) -> Self {
        Self::new(name, MenuType::Directory, parent_id, sort, route_path, None, None)
    }

    /// 创建菜单
    pub fn menu(
        name: String,
        parent_id: u64,
        sort: i32,
        route_path: String,
        component: String,
        permission: Option<String>,
    ) -> Self {
        Self::new(
            name,
            MenuType::Menu,
            parent_id,
            sort,
            Some(route_path),
            Some(component),
            permission,
        )
    }

    /// 创建按钮权限
    pub fn button(name: String, parent_id: u64, sort: i32, permission: String) -> Self {
        Self::new(name, MenuType::Button, parent_id, sort, None, None, Some(permission))
    }

    fn new(
        name: String,
        menu_type: MenuType,
        parent_id: u64,
        sort: i32,
        route_path: Option<String>,
        component: Option<String>,
        permission: Option<String>,
    ) -> Self {
        Self {
            id: 0,
            name,
            permission,
            menu_type,
            sort,
            parent_id,
            route_path,
            icon: None,
            component,
            component_name: None,
            status: MenuStatus::Active,
            visible: true,
            keep_alive: true,
            always_show: false,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 是否为目录
    pub fn is_directory(&self) -> bool {
        self.menu_type == MenuType::Directory
    }

    /// 是否为菜单
    pub fn is_menu(&self) -> bool {
        self.menu_type == MenuType::Menu
    }

    /// 是否为按钮
    pub fn is_button(&self) -> bool {
        self.menu_type == MenuType::Button
    }

    /// 是否为顶级菜单
    pub fn is_root(&self) -> bool {
        self.parent_id == 0
    }

    /// 是否可用
    pub fn is_active(&self) -> bool {
        self.status == MenuStatus::Active && self.deleted == 0
    }

    /// 软删除
    pub fn mark_deleted(&mut self) {
        self.deleted = 1;
    }
}

// ─── 仓储 trait ──────────────────────────────────────────────

/// 菜单仓储 trait
#[async_trait]
pub trait MenuRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Menu>, anyhow::Error>;
    async fn find_all(&self) -> Result<Vec<Menu>, anyhow::Error>;
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> Result<Vec<Menu>, anyhow::Error>;
    async fn find_menu_tree(&self) -> Result<Vec<Menu>, anyhow::Error>;
    async fn save(&self, menu: &Menu) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_by_permission(&self, permission: &str) -> Result<Option<Menu>, anyhow::Error>;
}
