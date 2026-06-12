use std::any::Any;
use async_trait::async_trait;
use tx_error::AppResult;
use crate::department::model::aggregate::Department;
use crate::department::model::value_object::DeptQuery;

#[async_trait]
pub trait DepartmentRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Department>>;
    async fn find_all(&self, query: &DeptQuery) -> AppResult<Vec<Department>>;
    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Department>>;
    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Department>>;
    async fn insert(&self, dept: &Department) -> AppResult<()>;
    async fn update(&self, dept: &Department) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn has_children(&self, parent_id: u64) -> AppResult<bool>;
    async fn has_users(&self, dept_id: u64) -> AppResult<bool>;
}
