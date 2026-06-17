use std::sync::Arc;
use tx_common::id;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::menu::model::aggregate::Menu;
use crate::shared::model::value_object::DeletedStatus;
use crate::menu::model::value_object::{MenuQuery, MenuTreeNode};
use crate::menu::repository::MenuRepository;

/// Menu domain service
#[tx_comp]
pub struct MenuService {
    menu_repo: Arc<dyn MenuRepository>,
}

impl MenuService {
    /// 创建菜单服务实例
    ///
    /// # 参数
    /// * `menu_repo` - 菜单仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(menu_repo: Arc<dyn MenuRepository>) -> Self {
        Self { menu_repo }
    }

    /// 创建新菜单
    ///
    /// # 参数
    /// * `name` - 菜单名称
    /// * `permission` - 菜单关联的权限标识，用于权限校验
    /// * `types` - 菜单类型（如目录、菜单、按钮等）
    /// * `sort` - 排序号，数值越小越靠前
    /// * `parent_id` - 父菜单 ID，顶级菜单传 0
    /// * `path` - 前端路由路径（可选）
    /// * `icon` - 菜单图标（可选）
    /// * `component` - 前端组件路径（可选）
    /// * `component_name` - 前端组件名称（可选）
    /// * `creator` - 创建人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 调用 `id::next_id()` 生成全局唯一菜单 ID
    /// 2. 通过聚合根 `Menu::create` 构造菜单实体
    /// 3. 设置可选字段（路径、图标、组件路径、组件名称）
    /// 4. 调用仓储的 `insert` 方法将菜单持久化到数据库
    ///
    /// # 返回
    /// 成功返回新创建的 `Menu` 聚合根实体
    ///
    /// # 错误
    /// - 数据库插入操作失败时返回仓储层错误
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

    /// 更新菜单信息
    ///
    /// # 参数
    /// * `menu_id` - 要更新的菜单 ID
    /// * `name` - 菜单名称
    /// * `permission` - 权限标识
    /// * `types` - 菜单类型
    /// * `sort` - 排序号
    /// * `parent_id` - 父菜单 ID
    /// * `path` - 路由路径（可选）
    /// * `icon` - 菜单图标（可选）
    /// * `component` - 前端组件路径（可选）
    /// * `component_name` - 前端组件名称（可选）
    /// * `visible` - 是否可见
    /// * `keep_alive` - 是否缓存组件
    /// * `updater` - 更新人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 根据 `menu_id` 从仓储查询菜单，不存在则抛出 `NotFoundMenu` 错误
    /// 2. 校验 `parent_id` 不能等于 `menu_id`（不允许将自身设为父级）
    /// 3. 调用聚合根 `update_info` 方法更新菜单属性
    /// 4. 调用仓储的 `update` 方法持久化变更
    ///
    /// # 返回
    /// 成功返回更新后的 `Menu` 聚合根实体
    ///
    /// # 错误
    /// - `NotFoundMenu` - 指定菜单 ID 不存在
    /// - `ValidationMenuSelfParent` - 尝试将菜单的父级设为自身
    /// - 数据库更新操作失败时返回仓储层错误
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
            .ok_or_else(|| RepositoryError::NotFoundMenu)?;

        // Cannot set self as parent
        if parent_id == menu_id {
            return Err(RepositoryError::ValidationMenuSelfParent)?;
        }

        menu.update_info(
            name, permission, types, sort, parent_id, path, icon, component, component_name,
            visible, keep_alive, updater,
        );
        self.menu_repo.update(&menu).await?;
        Ok(menu)
    }

    /// 删除菜单（软删除）
    ///
    /// # 参数
    /// * `menu_id` - 要删除的菜单 ID
    /// * `updater` - 操作人标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 检查该菜单是否存在子菜单，若存在则拒绝删除
    /// 2. 根据 `menu_id` 查询菜单实体，不存在则抛出 `NotFoundMenu` 错误
    /// 3. 调用聚合根 `soft_delete` 方法标记为已删除
    /// 4. 调用仓储的 `update` 方法持久化删除状态
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `ValidationMenuHasChildren` - 该菜单下存在子菜单，不允许删除
    /// - `NotFoundMenu` - 指定菜单 ID 不存在
    /// - 数据库更新操作失败时返回仓储层错误
    pub async fn delete_menu(
        &self,
        menu_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        if self.menu_repo.has_children(menu_id).await? {
            return Err(RepositoryError::ValidationMenuHasChildren)?;
        }

        let mut menu = self
            .menu_repo
            .find_by_id(menu_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundMenu)?;

        menu.soft_delete(updater);
        self.menu_repo.update(&menu).await?;
        Ok(())
    }

    /// 根据查询条件获取所有菜单列表
    ///
    /// # 参数
    /// * `query` - 菜单查询条件，包含筛选和排序参数
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_all` 方法，根据查询条件检索菜单列表
    ///
    /// # 返回
    /// 成功返回匹配条件的 `Menu` 实体列表
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_all_menus(&self, query: &MenuQuery) -> AppResult<Vec<Menu>> {
        self.menu_repo.find_all(query).await
    }

    /// 根据 ID 列表批量获取菜单
    ///
    /// # 参数
    /// * `ids` - 菜单 ID 切片，支持批量查询
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_by_ids` 方法，根据 ID 列表批量检索菜单
    ///
    /// # 返回
    /// 成功返回与给定 ID 匹配的 `Menu` 实体列表（不保证顺序）
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_menus_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>> {
        self.menu_repo.find_by_ids(ids).await
    }

    /// 获取菜单树形结构
    ///
    /// # 参数
    /// * `query` - 菜单查询条件，用于筛选参与构建树的菜单数据
    ///
    /// # 执行逻辑
    /// 1. 调用仓储的 `find_all` 方法获取满足条件的全部菜单列表
    /// 2. 调用 `build_tree` 递归方法，以 `parent_id = 0` 为根节点构建树形结构
    ///
    /// # 返回
    /// 成功返回 `MenuTreeNode` 树形结构列表，每个节点包含其子节点
    ///
    /// # 错误
    /// - 数据库查询操作失败时返回仓储层错误
    pub async fn get_menu_tree(&self, query: &MenuQuery) -> AppResult<Vec<MenuTreeNode>> {
        let menus = self.menu_repo.find_all(query).await?;
        Ok(Self::build_tree(&menus, 0))
    }

    /// 递归构建菜单树（内部方法）
    ///
    /// 筛选未删除且 `parent_id` 匹配的菜单，递归组装子节点形成树形结构。
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
