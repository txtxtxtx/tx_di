use crate::identity::department::model::aggregate::Department;
use crate::identity::department::model::value_object::DeptQuery;
use async_trait::async_trait;
use std::any::Any;
use tx_error::{AppResult, CodeMsg};

/// Department 仓储错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum DepartmentRepositoryError {
    #[err(10003, "数据库异常")]
    DatabaseDept,
    #[err(10103, "记录不存在")]
    NotFoundDept,
    #[err(10303, "部门已禁用，无法分配")]
    ValidationDeptDisabled,
    #[err(10309, "部门下存在子部门，无法删除")]
    ValidationDeptHasChildren,
    #[err(10310, "部门下存在用户，无法删除")]
    ValidationDeptHasUsers,
    #[err(10311, "部门不能将自身设为上级部门")]
    ValidationDeptSelfParent,
}

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
