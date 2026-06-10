use std::sync::Arc;

use crate::menu::dto::*;
use admin_domain::menu::model::value_object::{MenuQuery, MenuTreeNode};
use admin_domain::menu::service::MenuService;
use admin_domain::shared::repository::RepositoryError;

pub struct MenuAppService {
    menu_service: Arc<MenuService>,
}

impl MenuAppService {
    pub fn new(menu_service: Arc<MenuService>) -> Self {
        Self { menu_service }
    }

    pub async fn create_menu(
        &self,
        cmd: CreateMenuCommand,
        creator: Option<String>,
    ) -> Result<MenuResponse, RepositoryError> {
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

    pub async fn update_menu(
        &self,
        cmd: UpdateMenuCommand,
        updater: Option<String>,
    ) -> Result<MenuResponse, RepositoryError> {
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

    pub async fn delete_menu(&self, menu_id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        self.menu_service.delete_menu(menu_id, updater).await
    }

    pub async fn get_menu_list(
        &self,
        request: MenuQueryRequest,
    ) -> Result<Vec<MenuResponse>, RepositoryError> {
        let query = MenuQuery {
            name: request.name,
            status: request.status,
            types: request.types,
        };
        let menus = self.menu_service.get_all_menus(&query).await?;
        Ok(menus.into_iter().map(MenuResponse::from).collect())
    }

    pub async fn get_menu_tree(
        &self,
        request: MenuQueryRequest,
    ) -> Result<Vec<MenuTreeNode>, RepositoryError> {
        let query = MenuQuery {
            name: request.name,
            status: request.status,
            types: request.types,
        };
        self.menu_service.get_menu_tree(&query).await
    }
}
