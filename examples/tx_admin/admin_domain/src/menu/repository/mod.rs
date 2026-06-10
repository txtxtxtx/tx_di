use async_trait::async_trait;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::menu::model::aggregate::Menu;
use crate::menu::model::value_object::MenuQuery;

/// Menu repository trait
#[async_trait]
pub trait MenuRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Menu>>;
    async fn find_all(&self, query: &MenuQuery) -> AppResult<Vec<Menu>>;
    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>>;
    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Menu>>;
    async fn insert(&self, menu: &Menu) -> AppResult<()>;
    async fn update(&self, menu: &Menu) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn has_children(&self, parent_id: u64) -> AppResult<bool>;
}
