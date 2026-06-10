use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::menu::model::aggregate::Menu;
use admin_domain::menu::model::value_object::MenuQuery;
use admin_domain::menu::repository::MenuRepository;
use admin_domain::shared::repository::RepositoryError;
use tx_error::AppResult;

pub struct MockMenuRepository {
    menus: RwLock<HashMap<u64, Menu>>,
}

impl MockMenuRepository {
    pub fn new() -> Self {
        Self {
            menus: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_menu(self, menu: Menu) -> Self {
        {
            let mut menus = self.menus.write().unwrap();
            menus.insert(menu.id, menu);
        }
        self
    }
}

impl Default for MockMenuRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MenuRepository for MockMenuRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Menu>> {
        let menus = self.menus.read().unwrap();
        Ok(menus.get(&id).filter(|m| m.audit.deleted == 0).cloned())
    }

    async fn find_all(&self, query: &MenuQuery) -> AppResult<Vec<Menu>> {
        let menus = self.menus.read().unwrap();
        Ok(menus
            .values()
            .filter(|m| m.audit.deleted == 0)
            .filter(|m| {
                if let Some(ref name) = query.name {
                    if !m.name.contains(name.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if m.status != status {
                        return false;
                    }
                }
                if let Some(types) = query.types {
                    if m.types != types {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }

    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>> {
        let menus = self.menus.read().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| menus.get(id))
            .filter(|m| m.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Menu>> {
        let menus = self.menus.read().unwrap();
        Ok(menus
            .values()
            .filter(|m| m.parent_id == parent_id && m.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn insert(&self, menu: &Menu) -> AppResult<()> {
        let mut menus = self.menus.write().unwrap();
        menus.insert(menu.id, menu.clone());
        Ok(())
    }

    async fn update(&self, menu: &Menu) -> AppResult<()> {
        let mut menus = self.menus.write().unwrap();
        menus.insert(menu.id, menu.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut menus = self.menus.write().unwrap();
        if let Some(menu) = menus.get_mut(&id) {
            menu.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn has_children(&self, parent_id: u64) -> AppResult<bool> {
        let menus = self.menus.read().unwrap();
        Ok(menus
            .values()
            .any(|m| m.parent_id == parent_id && m.audit.deleted == 0))
    }
}
