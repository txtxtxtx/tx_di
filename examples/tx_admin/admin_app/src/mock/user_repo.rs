use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::UserQuery;
use admin_domain::user::repository::UserRepository;
use admin_domain::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_error::AppResult;

/// Mock user repository for testing
pub struct MockUserRepository {
    users: RwLock<HashMap<u64, User>>,
    user_roles: RwLock<HashMap<u64, Vec<u64>>>,
    user_depts: RwLock<HashMap<u64, Vec<u64>>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            user_roles: RwLock::new(HashMap::new()),
            user_depts: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_user(self, user: User) -> Self {
        {
            let mut users = self.users.write().unwrap();
            users.insert(user.id, user);
        }
        self
    }
}

impl Default for MockUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<User>> {
        let users = self.users.read().unwrap();
        Ok(users.get(&id).filter(|u| u.audit.deleted == 0).cloned())
    }

    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let users = self.users.read().unwrap();
        Ok(users
            .values()
            .find(|u| u.username == username && u.audit.deleted == 0)
            .cloned())
    }

    async fn find_page(
        &self,
        query: &UserQuery,
        page: Page<User>,
    ) -> AppResult<Page<User>> {
        let users = self.users.read().unwrap();
        let filtered: Vec<User> = users
            .values()
            .filter(|u| u.audit.deleted == 0)
            .filter(|u| {
                if let Some(ref username) = query.username {
                    if !u.username.contains(username.as_str()) {
                        return false;
                    }
                }
                if let Some(ref nickname) = query.nickname {
                    if !u.nickname.contains(nickname.as_str()) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if u.status != status {
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
            .take(page.size as usize)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn find_all(&self, query: &UserQuery) -> AppResult<Vec<User>> {
        let users = self.users.read().unwrap();
        Ok(users
            .values()
            .filter(|u| u.audit.deleted == 0)
            .filter(|u| {
                if let Some(status) = query.status {
                    if u.status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }

    async fn insert(&self, user: &User) -> AppResult<()> {
        let mut users = self.users.write().unwrap();
        if users.contains_key(&user.id) {
            return Err(RepositoryError::Duplicate)?;
        }
        users.insert(user.id, user.clone());
        Ok(())
    }

    async fn update(&self, user: &User) -> AppResult<()> {
        let mut users = self.users.write().unwrap();
        if !users.contains_key(&user.id) {
            return Err(RepositoryError::NotFound)?;
        }
        users.insert(user.id, user.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut users = self.users.write().unwrap();
        if let Some(user) = users.get_mut(&id) {
            user.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn exists_by_username(&self, username: &str) -> AppResult<bool> {
        let users = self.users.read().unwrap();
        Ok(users
            .values()
            .any(|u| u.username == username && u.audit.deleted == 0))
    }

    async fn count(&self, _query: &UserQuery) -> AppResult<i64> {
        let users = self.users.read().unwrap();
        Ok(users.values().filter(|u| u.audit.deleted == 0).count() as i64)
    }

    async fn find_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>> {
        let user_roles = self.user_roles.read().unwrap();
        let users = self.users.read().unwrap();
        let user_ids: Vec<u64> = user_roles
            .iter()
            .filter(|(_, roles)| roles.contains(&role_id))
            .map(|(id, _)| *id)
            .collect();

        Ok(user_ids
            .iter()
            .filter_map(|id| users.get(id))
            .filter(|u| u.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn find_by_dept_id(&self, dept_id: u64) -> AppResult<Vec<User>> {
        let user_depts = self.user_depts.read().unwrap();
        let users = self.users.read().unwrap();
        let user_ids: Vec<u64> = user_depts
            .iter()
            .filter(|(_, depts)| depts.contains(&dept_id))
            .map(|(id, _)| *id)
            .collect();

        Ok(user_ids
            .iter()
            .filter_map(|id| users.get(id))
            .filter(|u| u.audit.deleted == 0)
            .cloned()
            .collect())
    }

    async fn bind_roles(&self, user_id: u64, role_ids: &[u64]) -> AppResult<()> {
        let mut user_roles = self.user_roles.write().unwrap();
        user_roles.insert(user_id, role_ids.to_vec());
        Ok(())
    }

    async fn bind_departments(&self, user_id: u64, dept_ids: &[u64]) -> AppResult<()> {
        let mut user_depts = self.user_depts.write().unwrap();
        user_depts.insert(user_id, dept_ids.to_vec());
        Ok(())
    }

    async fn get_role_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        let user_roles = self.user_roles.read().unwrap();
        Ok(user_roles.get(&user_id).cloned().unwrap_or_default())
    }

    async fn get_dept_ids(&self, user_id: u64) -> AppResult<Vec<u64>> {
        let user_depts = self.user_depts.read().unwrap();
        Ok(user_depts.get(&user_id).cloned().unwrap_or_default())
    }
}
