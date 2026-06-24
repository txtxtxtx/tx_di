//! 角色管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::role::role_service_server::RoleService;
use admin_proto::admin::role::{
    AddUsersToRoleRequest, AssignMenusRequest, CreateRoleRequest, DeleteRoleRequest,
    GetRoleRequest, GetRoleUsersRequest, GetRoleUsersResponse, ListRolesRequest,
    ListRolesResponse, RemoveUsersFromRoleRequest, RoleResponse, UpdateRoleRequest,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct RoleGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl RoleService for RoleGrpcService {
    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:create").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        let r = svc.create_role(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_role(
        &self,
        request: Request<UpdateRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:update").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        let r = svc.update_role(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_role(
        &self,
        request: Request<DeleteRoleRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        svc.delete_role(req.role_id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_role(
        &self,
        request: Request<GetRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        let r = svc.get_role(req.role_id).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn list_roles(
        &self,
        request: Request<ListRolesRequest>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        let p = svc.get_role_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListRolesResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn assign_menus(
        &self,
        request: Request<AssignMenusRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:assign_menu").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        svc.assign_menus(req.role_id, req.menu_ids).await.map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_role_users(
        &self,
        request: Request<GetRoleUsersRequest>,
    ) -> Result<Response<GetRoleUsersResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        let users = svc
            .get_role_users(req.role_id)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(GetRoleUsersResponse { users }))
    }

    async fn add_users_to_role(
        &self,
        request: Request<AddUsersToRoleRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:assign_menu").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        svc.add_users_to_role(req.role_id, req.user_ids)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn remove_users_from_role(
        &self,
        request: Request<RemoveUsersFromRoleRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "role:assign_menu").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::role::app_service::RoleAppService> = self.app.inject();
        svc.remove_users_from_role(req.role_id, req.user_ids)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }
}
