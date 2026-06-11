//! 角色管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::role::role_service_server::RoleService;
use admin_proto::admin::role::{
    CreateRoleRequest, RoleResponse, UpdateRoleRequest, DeleteRoleRequest,
    GetRoleRequest, ListRolesRequest, ListRolesResponse, AssignMenusRequest,
};
use admin_proto::Empty;

/// 角色 gRPC 服务
#[derive(Debug, Default)]
pub struct RoleGrpcService;

#[tonic::async_trait]
impl RoleService for RoleGrpcService {
    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 RoleAppService::create
        let resp = RoleResponse {
            id: 1,
            name: req.name.clone(),
            code: req.code.clone(),
            sort: req.sort,
            data_scope: 0,
            status: 1,
            remark: req.remark.clone(),
            menu_ids: req.menu_ids.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn update_role(
        &self,
        request: Request<UpdateRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 RoleAppService::update
        let resp = RoleResponse {
            id: req.role_id,
            name: req.name.clone(),
            code: req.code.clone(),
            sort: req.sort,
            data_scope: req.data_scope,
            status: 1,
            remark: req.remark.clone(),
            menu_ids: vec![],
        };
        Ok(Response::new(resp))
    }

    async fn delete_role(
        &self,
        request: Request<DeleteRoleRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 RoleAppService::delete
        let _ = req.role_id;
        Ok(Response::new(Empty {}))
    }

    async fn get_role(
        &self,
        request: Request<GetRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 RoleAppService::get_by_id
        let resp = RoleResponse {
            id: req.role_id,
            name: "placeholder".into(),
            code: "placeholder".into(),
            sort: 0,
            data_scope: 0,
            status: 1,
            remark: None,
            menu_ids: vec![],
        };
        Ok(Response::new(resp))
    }

    async fn list_roles(
        &self,
        request: Request<ListRolesRequest>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 调用 RoleAppService::list
        let resp = ListRolesResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }

    async fn assign_menus(
        &self,
        _request: Request<AssignMenusRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 RoleAppService::assign_menus
        Ok(Response::new(Empty {}))
    }
}
