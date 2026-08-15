use crate::identity::role::model::aggregate::Role;
use crate::identity::role::model::value_object::RoleQuery;
use crate::identity::user::model::aggregate::User;
use async_trait::async_trait;
use std::any::Any;
use tx_common::page::Page;
use tx_error::{AppResult, CodeMsg};

/// Role 仓储错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum RoleRepositoryError {
    #[err(10002, "数据库异常")]
    DatabaseRole,
    #[err(10102, "记录不存在")]
    NotFoundRole,
    #[err(10202, "角色编码已存在")]
    DuplicateRoleCode,
    #[err(10302, "角色已禁用，无法分配")]
    ValidationRoleDisabled,
}

/// Role repository trait
#[async_trait]
pub trait RoleRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Role>>;
    async fn find_by_code(&self, code: &str) -> AppResult<Option<Role>>;
    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Role>>;
    async fn find_page(&self, query: &RoleQuery, page: Page<Role>) -> AppResult<Page<Role>>;
    async fn find_all(&self, query: &RoleQuery) -> AppResult<Vec<Role>>;
    async fn insert(&self, role: &Role) -> AppResult<()>;
    async fn update(&self, role: &Role) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn exists_by_code(&self, code: &str) -> AppResult<bool>;

    /// 原子创建角色并绑定菜单（同一数据库事务）
    ///
    /// 用于"建角色 + 绑菜单"场景，任一步失败整体回滚，
    /// 避免绑定失败时留下孤儿角色。
    async fn create_role_with_menus(&self, role: &Role, menu_ids: &[u64]) -> AppResult<()>;
    async fn bind_menus(&self, role_id: u64, menu_ids: &[u64]) -> AppResult<()>;
    async fn get_menu_ids(&self, role_id: u64) -> AppResult<Vec<u64>>;
    async fn get_user_ids(&self, role_id: u64) -> AppResult<Vec<u64>>;
    async fn find_users_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>>;
    async fn bind_users(&self, role_id: u64, user_ids: &[u64]) -> AppResult<()>;
    async fn unbind_users(&self, role_id: u64, user_ids: &[u64]) -> AppResult<()>;
}
