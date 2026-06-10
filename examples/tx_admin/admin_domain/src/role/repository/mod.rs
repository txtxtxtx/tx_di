use async_trait::async_trait;
use tx_common::page::Page;
use tx_error::AppResult;
use crate::role::model::aggregate::Role;
use crate::role::model::value_object::RoleQuery;

/// Role repository trait
#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Role>>;
    async fn find_by_code(&self, code: &str) -> AppResult<Option<Role>>;
    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>>;
    async fn find_page(
        &self,
        query: &RoleQuery,
        page: Page<Role>,
    ) -> AppResult<Page<Role>>;
    async fn find_all(&self, query: &RoleQuery) -> AppResult<Vec<Role>>;
    async fn insert(&self, role: &Role) -> AppResult<()>;
    async fn update(&self, role: &Role) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn exists_by_code(&self, code: &str) -> AppResult<bool>;
    async fn bind_menus(&self, role_id: u64, menu_ids: &[u64]) -> AppResult<()>;
    async fn get_menu_ids(&self, role_id: u64) -> AppResult<Vec<u64>>;
}
