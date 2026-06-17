use std::sync::Arc;

use crate::menu::dto::*;
use admin_domain::menu::model::value_object::{MenuQuery, MenuTreeNode};
use admin_domain::menu::service::MenuService;
use tx_di_core::tx_comp;
use tx_error::AppResult;

#[tx_comp]
pub struct MenuAppService {
    menu_service: Arc<MenuService>,
}

impl MenuAppService {
    /// 创建菜单应用服务实例
    ///
    /// # 参数
    /// * `menu_service` - 菜单领域服务，用于执行菜单相关的业务逻辑
    pub fn new(menu_service: Arc<MenuService>) -> Self {
        Self { menu_service }
    }

    /// 创建新菜单
    ///
    /// # 参数
    /// * `cmd` - 创建菜单命令，包含菜单名称、权限标识、菜单类型、排序号、父菜单ID、路由路径、图标、组件路径、组件名称
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给菜单领域服务执行创建操作，逻辑详见 `MenuService::create_menu`
    ///
    /// # 返回
    /// 成功返回 `MenuResponse`，包含菜单完整信息
    ///
    /// # 错误
    /// - `NotFoundMenu` - 父菜单ID对应的菜单不存在
    /// - 数据库写入异常
    pub async fn create_menu(
        &self,
        cmd: CreateMenuCommand,
        creator: Option<String>,
    ) -> AppResult<MenuResponse> {
        let menu = self
            .menu_service
            .create_menu(
                cmd.name,
                cmd.permission,
                cmd.types,
                cmd.sort,
                cmd.parent_id,
                cmd.path,
                cmd.icon,
                cmd.component,
                cmd.component_name,
                creator,
            )
            .await?;
        Ok(MenuResponse::from(menu))
    }

    /// 更新菜单信息
    ///
    /// # 参数
    /// * `cmd` - 更新菜单命令，包含菜单ID、名称、权限标识、菜单类型、排序号、父菜单ID、路由路径、图标、组件路径、组件名称、是否可见、是否缓存
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给菜单领域服务执行更新操作，逻辑详见 `MenuService::update_menu`
    ///
    /// # 返回
    /// 成功返回更新后的 `MenuResponse`
    ///
    /// # 错误
    /// - `NotFoundMenu` - 菜单ID对应的菜单不存在
    /// - 数据库更新异常
    pub async fn update_menu(
        &self,
        cmd: UpdateMenuCommand,
        updater: Option<String>,
    ) -> AppResult<MenuResponse> {
        let menu = self
            .menu_service
            .update_menu(
                cmd.menu_id,
                cmd.name,
                cmd.permission,
                cmd.types,
                cmd.sort,
                cmd.parent_id,
                cmd.path,
                cmd.icon,
                cmd.component,
                cmd.component_name,
                cmd.visible,
                cmd.keep_alive,
                updater,
            )
            .await?;
        Ok(MenuResponse::from(menu))
    }

    /// 删除菜单
    ///
    /// # 参数
    /// * `menu_id` - 要删除的菜单ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给菜单领域服务执行删除操作，逻辑详见 `MenuService::delete_menu`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundMenu` - 菜单ID对应的菜单不存在
    /// - 存在子菜单时可能拒绝删除
    /// - 数据库删除异常
    pub async fn delete_menu(&self, menu_id: u64, updater: Option<String>) -> AppResult<()> {
        self.menu_service.delete_menu(menu_id, updater).await
    }

    /// 获取菜单列表（扁平结构）
    ///
    /// # 参数
    /// * `request` - 查询请求，包含菜单名称、状态、菜单类型等筛选条件
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `MenuQuery`
    /// 2. 委托给菜单领域服务查询所有符合条件的菜单
    /// 3. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Vec<MenuResponse>`，包含所有符合条件的菜单列表（扁平结构）
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_menu_list(
        &self,
        request: MenuQueryRequest,
    ) -> AppResult<Vec<MenuResponse>> {
        let query = MenuQuery {
            name: request.name,
            status: request.status,
            types: request.types,
        };
        let menus = self.menu_service.get_all_menus(&query).await?;
        Ok(menus.into_iter().map(MenuResponse::from).collect())
    }

    /// 获取菜单树结构
    ///
    /// # 参数
    /// * `request` - 查询请求，包含菜单名称、状态、菜单类型等筛选条件
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `MenuQuery`
    /// 2. 委托给菜单领域服务构建菜单树结构
    ///
    /// # 返回
    /// 成功返回 `Vec<MenuTreeNode>`，包含树形结构的菜单列表
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_menu_tree(
        &self,
        request: MenuQueryRequest,
    ) -> AppResult<Vec<MenuTreeNode>> {
        let query = MenuQuery {
            name: request.name,
            status: request.status,
            types: request.types,
        };
        self.menu_service.get_menu_tree(&query).await
    }
}
