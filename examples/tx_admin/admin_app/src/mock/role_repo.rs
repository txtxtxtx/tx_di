use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::role::model::aggregate::Role;
use admin_domain::role::model::value_object::RoleQuery;
use admin_domain::role::repository::RoleRepository;
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

pub struct MockRoleRepository {
    roles: RwLock<HashMap<u64, Role>>,
    role_menus: RwLock<HashMap<u64, Vec<u64>>>,
}

impl MockRoleRepository {
    pub fn new() -> Self {
        Self {
            roles: RwLock::new(HashMap::new()),
            role_menus: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_role(self, role: Role) -> Self {
        {
            let mut roles = self.roles.write().unwrap();
            roles.insert(role.id, role);
        }
        self
    }
}

impl Default for MockRoleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoleRepository for MockRoleRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Role>, RepositoryError> {
        let roles = self.roles.read().unwrap();
        Ok(roles.get(&id).filter(|r| r.audit.deleted == 0).cloned())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Role>, RepositoryError> {
        let roles = self.roles.read().unwrap();
        Ok(roles
            .values()
            .find(|r| r.code == code && r.audit.deleted == 0)
            .cloned())
    }

    async fn find_by_ids(&self, ids: &[u64]) -> Result<Vec<Role>, RepositoryError> {
        let roles = self.roles.read().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| roles.get(id))
            .filter(|r| r.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn find_page(
        &self,
        query: &RoleQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<Role>, RepositoryError> {
        let roles = self.roles.read().unwrap();
        let filtered: Vec<Role> = roles
            .values()
            .filter(|r| r.audit.deleted == 0)
            .filter(|r| {
                if let Some(ref name) = query.name {
                    if !r.name.contains(name.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if r.status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let list = filtered
            .into_iter()
            .skip(offset)
            .take(page.page_size as usize)
            .collect();

        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn find_all(&self, query: &RoleQuery) -> Result<Vec<Role>, RepositoryError> {
        let roles = self.roles.read().unwrap();
        Ok(roles
            .values()
            .filter(|r| r.audit.deleted == 0)
            .filter(|r| {
                if let Some(status) = query.status {
                    if r.status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }

    async fn insert(&self, role: &Role) -> Result<(), RepositoryError> {
        let mut roles = self.roles.write().unwrap();
        roles.insert(role.id, role.clone());
        Ok(())
    }

    async fn update(&self, role: &Role) -> Result<(), RepositoryError> {
        let mut roles = self.roles.write().unwrap();
        roles.insert(role.id, role.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError> {
        let mut roles = self.roles.write().unwrap();
        if let Some(role) = roles.get_mut(&id) {
            role.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!("Role {} not found", id)))
        }
    }

    async fn exists_by_code(&self, code: &str) -> Result<bool, RepositoryError> {
        let roles = self.roles.read().unwrap();
        Ok(roles
            .values()
            .any(|r| r.code == code && r.audit.deleted == 0))
    }

    async fn bind_menus(&self, role_id: u64, menu_ids: &[u64]) -> Result<(), RepositoryError> {
        let mut role_menus = self.role_menus.write().unwrap();
        role_menus.insert(role_id, menu_ids.to_vec());
        Ok(())
    }

    async fn get_menu_ids(&self, role_id: u64) -> Result<Vec<u64>, RepositoryError> {
        let role_menus = self.role_menus.read().unwrap();
        Ok(role_menus.get(&role_id).cloned().unwrap_or_default())
    }
}
