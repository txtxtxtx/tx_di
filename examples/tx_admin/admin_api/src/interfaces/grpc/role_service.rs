//! 角色管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::role::role_service_server::RoleService;
use admin_proto::admin::role::{
    CreateRoleRequest, RoleResponse, UpdateRoleRequest, DeleteRoleRequest,
    GetRoleRequest, ListRolesRequest, ListRolesResponse, AssignMenusRequest,
    GetRoleUsersRequest, GetRoleUsersResponse, AddUsersToRoleRequest, RemoveUsersFromRoleRequest,
};
use admin_proto::Empty;
use admin_proto::admin::common::PageResponse;

#[derive(Debug, Default)]
pub struct RoleGrpcService;

#[tonic::async_trait]
impl RoleService for RoleGrpcService {
    async fn create_role(&self, request: Request<CreateRoleRequest>) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        services::get().role.create_role(req, None).await
            .map(|r| Response::new(r))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_role(&self, request: Request<UpdateRoleRequest>) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        services::get().role.update_role(req, None).await
            .map(|r| Response::new(r))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_role(&self, request: Request<DeleteRoleRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().role.delete_role(req.role_id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_role(&self, request: Request<GetRoleRequest>) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        services::get().role.get_role(req.role_id).await
            .map(|r| Response::new(r))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_roles(&self, request: Request<ListRolesRequest>) -> Result<Response<ListRolesResponse>, Status> {
        let req = request.into_inner();
        services::get().role.get_role_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list;
                Response::new(ListRolesResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn assign_menus(&self, request: Request<AssignMenusRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().role.assign_menus(req).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_role_users(&self, request: Request<GetRoleUsersRequest>) -> Result<Response<GetRoleUsersResponse>, Status> {
        let req = request.into_inner();
        services::get().role.get_role_users(req.role_id).await
            .map(|users| Response::new(GetRoleUsersResponse { users }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn add_users_to_role(&self, request: Request<AddUsersToRoleRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().role.add_users_to_role(req.role_id, req.user_ids).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn remove_users_from_role(&self, request: Request<RemoveUsersFromRoleRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().role.remove_users_from_role(req.role_id, req.user_ids).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
