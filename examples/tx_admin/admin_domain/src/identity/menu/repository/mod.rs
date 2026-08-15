use crate::identity::menu::model::aggregate::Menu;
use crate::identity::menu::model::value_object::MenuQuery;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashSet;
use tx_error::{AppResult, CodeMsg};

/// Menu 仓储错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum MenuRepositoryError {
    #[err(10004, "数据库异常")]
    DatabaseMenu,
    #[err(10104, "记录不存在")]
    NotFoundMenu,
    #[err(10307, "菜单下存在子菜单，无法删除")]
    ValidationMenuHasChildren,
    #[err(10308, "菜单不能将自身设为上级菜单")]
    ValidationMenuSelfParent,
}

/// Menu repository trait
#[async_trait]
pub trait MenuRepository: Any + Send + Sync {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Menu>>;
    async fn find_all(&self, query: &MenuQuery) -> AppResult<Vec<Menu>>;
    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>>;
    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Menu>>;
    async fn insert(&self, menu: &Menu) -> AppResult<()>;
    async fn update(&self, menu: &Menu) -> AppResult<()>;
    async fn soft_delete(&self, id: u64) -> AppResult<()>;
    async fn has_children(&self, parent_id: u64) -> AppResult<bool>;

    /// 获取用户的权限码集合（通过角色关联的菜单中 types==2 的 permission 字段）
    async fn find_permission_codes_by_user_id(&self, user_id: u64) -> AppResult<HashSet<String>>;
}
