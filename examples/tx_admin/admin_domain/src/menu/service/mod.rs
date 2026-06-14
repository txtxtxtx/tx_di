use std::sync::Arc;
use tx_common::id;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::menu::model::aggregate::Menu;
use crate::shared::model::value_object::DeletedStatus;
use crate::menu::model::value_object::{MenuQuery, MenuTreeNode};
use crate::menu::repository::MenuRepository;
use crate::shared::repository::RepositoryError::NotFound;

/// Menu domain service
#[tx_comp]
pub struct MenuService {
    menu_repo: Arc<dyn MenuRepository>,
}

impl MenuService {
    pub fn new(menu_repo: Arc<dyn MenuRepository>) -> Self {
        Self { menu_repo }
    }

    /// Create a new menu
    pub async fn create_menu(
        &self,
        name: String,
        permission: String,
        types: i32,
        sort: i32,
        parent_id: u64,
        path: Option<String>,
        icon: Option<String>,
        component: Option<String>,
        component_name: Option<String>,
        creator: Option<String>,
    ) -> AppResult<Menu> {
        let menu_id = id::next_id();
        let mut menu = Menu::create(menu_id, name, permission, types, sort, parent_id, creator);
        menu.path = path;
        menu.icon = icon;
        menu.component = component;
        menu.component_name = component_name;
        self.menu_repo.insert(&menu).await?;
        Ok(menu)
    }

    /// Update menu
    pub async fn update_menu(
        &self,
        menu_id: u64,
        name: String,
        permission: String,
        types: i32,
        sort: i32,
        parent_id: u64,
        path: Option<String>,
        icon: Option<String>,
        component: Option<String>,
        component_name: Option<String>,
        visible: i32,
        keep_alive: i32,
        updater: Option<String>,
    ) -> AppResult<Menu> {
        let mut menu = self
            .menu_repo
            .find_by_id(menu_id)
            .await?
            .ok_or_else(|| NotFound)?;

        // Cannot set self as parent
        if parent_id == menu_id {
            return Err(RepositoryError::Validation)?;
        }

        menu.update_info(
            name, permission, types, sort, parent_id, path, icon, component, component_name,
            visible, keep_alive, updater,
        );
        self.menu_repo.update(&menu).await?;
        Ok(menu)
    }

    /// Delete menu
    pub async fn delete_menu(
        &self,
        menu_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        if self.menu_repo.has_children(menu_id).await? {
            return Err(RepositoryError::Validation)?;
        }

        let mut menu = self
            .menu_repo
            .find_by_id(menu_id)
            .await?
            .ok_or_else(|| NotFound)?;

        menu.soft_delete(updater);
        self.menu_repo.update(&menu).await?;
        Ok(())
    }

    /// Get all menus
    pub async fn get_all_menus(&self, query: &MenuQuery) -> AppResult<Vec<Menu>> {
        self.menu_repo.find_all(query).await
    }

    /// Get menus by IDs
    pub async fn get_menus_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>> {
        self.menu_repo.find_by_ids(ids).await
    }

    /// Get menu tree
    pub async fn get_menu_tree(&self, query: &MenuQuery) -> AppResult<Vec<MenuTreeNode>> {
        let menus = self.menu_repo.find_all(query).await?;
        Ok(Self::build_tree(&menus, 0))
    }

    /// Build menu tree recursively
    fn build_tree(menus: &[Menu], parent_id: u64) -> Vec<MenuTreeNode> {
        menus
            .iter()
            .filter(|m| m.parent_id == parent_id && m.audit.deleted == DeletedStatus::Normal)
            .map(|m| MenuTreeNode {
                id: m.id,
                name: m.name.clone(),
                permission: m.permission.clone(),
                types: m.types,
                sort: m.sort,
                parent_id: m.parent_id,
                path: m.path.clone(),
                icon: m.icon.clone(),
                component: m.component.clone(),
                component_name: m.component_name.clone(),
                status: m.status,
                visible: m.visible,
                keep_alive: m.keep_alive,
                children: Self::build_tree(menus, m.id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
