//! 菜单聚合

use std::fmt::Display;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum MenuType {
    #[column(variant = 0)] Directory,
    #[column(variant = 1)] Menu,
    #[column(variant = 2)] Button,
}
impl Display for MenuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { MenuType::Directory => write!(f, "directory"), MenuType::Menu => write!(f, "menu"), MenuType::Button => write!(f, "button") }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum MenuStatus {
    #[column(variant = 0)] Active,
    #[column(variant = 1)] Disabled,
}
impl Display for MenuStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { MenuStatus::Active => write!(f, "active"), MenuStatus::Disabled => write!(f, "disabled") }
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub id: u64, pub name: String, pub permission: Option<String>, pub menu_type: MenuType,
    pub sort: i32, pub parent_id: u64, pub route_path: Option<String>, pub icon: Option<String>,
    pub component: Option<String>, pub component_name: Option<String>, pub status: MenuStatus,
    pub visible: bool, pub keep_alive: bool, pub always_show: bool,
    pub creator: Option<String>, pub updater: Option<String>,
    pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8,
}

impl Menu {
    pub fn directory(name: String, parent_id: u64, sort: i32, route_path: Option<String>) -> Self { Self::new_inner(name, MenuType::Directory, parent_id, sort, route_path, None, None) }
    pub fn menu(name: String, parent_id: u64, sort: i32, route_path: String, component: String, permission: Option<String>) -> Self { Self::new_inner(name, MenuType::Menu, parent_id, sort, Some(route_path), Some(component), permission) }
    pub fn button(name: String, parent_id: u64, sort: i32, permission: String) -> Self { Self::new_inner(name, MenuType::Button, parent_id, sort, None, None, Some(permission)) }
    fn new_inner(name: String, menu_type: MenuType, parent_id: u64, sort: i32, route_path: Option<String>, component: Option<String>, permission: Option<String>) -> Self {
        Self { id: 0, name, permission, menu_type, sort, parent_id, route_path, icon: None, component, component_name: None, status: MenuStatus::Active, visible: true, keep_alive: true, always_show: false, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 }
    }
    pub fn is_active(&self) -> bool { self.status == MenuStatus::Active && self.deleted == 0 }
    pub fn mark_deleted(&mut self) { self.deleted = 1; }
}

#[async_trait]
pub trait MenuRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Menu>, anyhow::Error>;
    async fn find_all(&self) -> Result<Vec<Menu>, anyhow::Error>;
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> Result<Vec<Menu>, anyhow::Error>;
    async fn find_menu_tree(&self) -> Result<Vec<Menu>, anyhow::Error>;
    async fn save(&self, menu: &Menu) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
