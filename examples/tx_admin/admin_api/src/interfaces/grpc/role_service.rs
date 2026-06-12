//! 角色管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::role::role_service_server::RoleService;
use admin_proto::admin::role::{
    CreateRoleRequest, RoleResponse, UpdateRoleRequest, DeleteRoleRequest,
    GetRoleRequest, ListRolesRequest, ListRolesResponse, AssignMenusRequest,
};
use admin_proto::Empty;
use admin_proto::admin::common::PageResponse;
use crate::services;

#[derive(Debug, Default)]
pub struct RoleGrpcService;

fn map_role(r: admin_app::role::dto::RoleResponse) -> RoleResponse {
    RoleResponse {
        id: r.id, name: r.name, code: r.code, sort: r.sort,
        data_scope: r.data_scope, status: r.status, remark: r.remark,
        menu_ids: r.menu_ids,
    }
}

#[tonic::async_trait]
impl RoleService for RoleGrpcService {
    async fn create_role(&self, request: Request<CreateRoleRequest>) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::role::dto::CreateRoleCommand {
            name: req.name, code: req.code, sort: req.sort,
            remark: req.remark,
            menu_ids: if req.menu_ids.is_empty() { None } else { Some(req.menu_ids) },
        };
        services::get().role.create_role(cmd, None).await
            .map(|r| Response::new(map_role(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_role(&self, request: Request<UpdateRoleRequest>) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::role::dto::UpdateRoleCommand {
            role_id: req.role_id, name: req.name, code: req.code,
            sort: req.sort, data_scope: req.data_scope, remark: req.remark,
        };
        services::get().role.update_role(cmd, None).await
            .map(|r| Response::new(map_role(r)))
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
            .map(|r| Response::new(map_role(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_roles(&self, request: Request<ListRolesRequest>) -> Result<Response<ListRolesResponse>, Status> {
        let req = request.into_inner();
        let query = admin_app::role::dto::RoleQueryRequest {
            name: req.name, code: req.code, status: req.status,
            page: req.page, size: req.page_size,
        };
        services::get().role.get_role_page(query).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                let items = p.list.into_iter().map(map_role).collect();
                Response::new(ListRolesResponse { items, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn assign_menus(&self, request: Request<AssignMenusRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::role::dto::AssignMenusCommand { role_id: req.role_id, menu_ids: req.menu_ids };
        services::get().role.assign_menus(cmd).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
