use std::sync::Arc;

use crate::role::dto::*;
use admin_domain::role::model::value_object::RoleQuery;
use admin_domain::role::service::RoleService;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct RoleAppService {
    role_service: Arc<RoleService>,
}

impl RoleAppService {
    pub fn new(role_service: Arc<RoleService>) -> Self {
        Self { role_service }
    }

    pub async fn create_role(
        &self,
        cmd: CreateRoleCommand,
        creator: Option<String>,
    ) -> AppResult<RoleResponse> {
        let mut role = self
            .role_service
            .create_role(cmd.name, cmd.code, cmd.sort, creator)
            .await?;

        if let Some(menu_ids) = cmd.menu_ids {
            role = self.role_service.assign_menus(role.id, menu_ids).await?;
        }

        Ok(RoleResponse::from(role))
    }

    pub async fn update_role(
        &self,
        cmd: UpdateRoleCommand,
        updater: Option<String>,
    ) -> AppResult<RoleResponse> {
        let role = self
            .role_service
            .update_role(cmd.role_id, cmd.name, cmd.code, cmd.sort, cmd.data_scope, cmd.remark, updater)
            .await?;
        Ok(RoleResponse::from(role))
    }

    pub async fn delete_role(&self, role_id: u64, updater: Option<String>) -> AppResult<()> {
        self.role_service.delete_role(role_id, updater).await
    }

    pub async fn change_status(
        &self,
        role_id: u64,
        status: i32,
        updater: Option<String>,
    ) -> AppResult<RoleResponse> {
        let role = self.role_service.change_status(role_id, status, updater).await?;
        Ok(RoleResponse::from(role))
    }

    pub async fn assign_menus(&self, cmd: AssignMenusCommand) -> AppResult<RoleResponse> {
        let role = self.role_service.assign_menus(cmd.role_id, cmd.menu_ids).await?;
        Ok(RoleResponse::from(role))
    }

    pub async fn get_role(&self, role_id: u64) -> AppResult<RoleResponse> {
        let role = self.role_service.get_role(role_id).await?;
        Ok(RoleResponse::from(role))
    }

    pub async fn get_role_page(
        &self,
        request: RoleQueryRequest,
    ) -> AppResult<Page<RoleResponse>> {
        let query = RoleQuery {
            name: request.name,
            code: request.code,
            status: request.status,
        };
        let page = Page::request(request.page, request.size);
        let result = self.role_service.get_role_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(RoleResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    pub async fn get_all_roles(&self) -> AppResult<Vec<RoleResponse>> {
        let roles = self.role_service.get_all_roles(&RoleQuery::default()).await?;
        Ok(roles.into_iter().map(RoleResponse::from).collect())
    }
}
