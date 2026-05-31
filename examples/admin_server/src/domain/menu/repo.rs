//! 菜单仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use super::{Menu, MenuType, MenuStatus, MenuRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_menu"]
pub struct MenuModel {
    #[key] #[auto] pub id: u64, pub name: String, #[default("".to_string())] pub permission: String,
    pub menu_type: MenuType, #[default(0i32)] pub sort: i32, #[default(0u64)] pub parent_id: u64,
    #[column("path")] #[default("".to_string())] pub route_path: String, #[default("".to_string())] pub icon: String,
    #[default("".to_string())] pub component: String, #[default("".to_string())] pub component_name: String,
    pub status: MenuStatus, #[default(true)] pub visible: bool, #[default(false)] pub keep_alive: bool,
    #[default(false)] pub always_show: bool, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<MenuModel> for Menu {
    fn from(m: MenuModel) -> Self {
        Self { id: m.id, name: m.name, permission: if m.permission.is_empty() { None } else { Some(m.permission) }, menu_type: m.menu_type, sort: m.sort, parent_id: m.parent_id, route_path: if m.route_path.is_empty() { None } else { Some(m.route_path) }, icon: if m.icon.is_empty() { None } else { Some(m.icon) }, component: if m.component.is_empty() { None } else { Some(m.component) }, component_name: if m.component_name.is_empty() { None } else { Some(m.component_name) }, status: m.status, visible: m.visible, keep_alive: m.keep_alive, always_show: m.always_show, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted }
    }
}

#[derive(Debug, Clone, Model)]
#[table = "system_role_menu"]
pub struct RoleMenuModel {
    #[key] #[auto] pub id: u64, #[index] pub role_id: u64, #[index] pub menu_id: u64,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyMenuRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl MenuRepository for ToastyMenuRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Menu>, anyhow::Error> { let mut db = self.toasty.db().clone(); match MenuModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Menu::from(m))), Err(_) => Ok(None) } }
    async fn find_all(&self) -> Result<Vec<Menu>, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = MenuModel::all().exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.deleted == 0).map(Menu::from).collect()) }
    async fn find_by_role_ids(&self, role_ids: &[u64]) -> Result<Vec<Menu>, anyhow::Error> {
        let mut db = self.toasty.db().clone(); let role_menus = RoleMenuModel::all().exec(&mut db).await?;
        let menu_ids: Vec<u64> = role_menus.iter().filter(|rm| role_ids.contains(&rm.role_id) && rm.deleted == 0).map(|rm| rm.menu_id).collect();
        if menu_ids.is_empty() { return Ok(Vec::new()); }
        let mut menus = Vec::new(); for mid in menu_ids { match MenuModel::get_by_id(&mut db, mid).await { Ok(m) => { if m.deleted == 0 { menus.push(Menu::from(m)); } } Err(_) => {} } } Ok(menus)
    }
    async fn find_menu_tree(&self) -> Result<Vec<Menu>, anyhow::Error> { self.find_all().await }
    async fn save(&self, menu: &Menu) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if menu.id == 0 { toasty::create!(MenuModel { name: menu.name.clone(), permission: menu.permission.clone().unwrap_or_default(), menu_type: menu.menu_type, sort: menu.sort, parent_id: menu.parent_id, route_path: menu.route_path.clone().unwrap_or_default(), icon: menu.icon.clone().unwrap_or_default(), component: menu.component.clone().unwrap_or_default(), component_name: menu.component_name.clone().unwrap_or_default(), status: menu.status, visible: menu.visible, keep_alive: menu.keep_alive, always_show: menu.always_show, creator: menu.creator.clone().unwrap_or_default(), updater: menu.updater.clone().unwrap_or_default() }).exec(&mut db).await?; }
        else { let mut m = MenuModel::get_by_id(&mut db, menu.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.name = menu.name.clone(); m.permission = menu.permission.clone().unwrap_or_default(); m.menu_type = menu.menu_type; m.sort = menu.sort; m.parent_id = menu.parent_id; m.route_path = menu.route_path.clone().unwrap_or_default(); m.icon = menu.icon.clone().unwrap_or_default(); m.component = menu.component.clone().unwrap_or_default(); m.component_name = menu.component_name.clone().unwrap_or_default(); m.status = menu.status; m.visible = menu.visible; m.keep_alive = menu.keep_alive; m.always_show = menu.always_show; m.creator = menu.creator.clone().unwrap_or_default(); m.updater = menu.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(())
    }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match MenuModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted = 1; m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
