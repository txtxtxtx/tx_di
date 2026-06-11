//! 部门管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::dept::department_service_server::DepartmentService;
use admin_proto::admin::dept::{
    CreateDeptRequest, DeptResponse, UpdateDeptRequest, DeleteDeptRequest,
    GetDeptRequest, ListDeptsRequest, ListDeptsResponse,
};
use admin_proto::Empty;
use crate::services;

#[derive(Debug, Default)]
pub struct DeptGrpcService;

fn map_dept(d: admin_app::department::dto::DeptResponse) -> DeptResponse {
    DeptResponse {
        id: d.id, name: d.name, parent_id: d.parent_id, sort: d.sort,
        leader_user_id: d.leader_user_id, phone: d.phone, email: d.email, status: d.status,
    }
}

#[tonic::async_trait]
impl DepartmentService for DeptGrpcService {
    async fn create_dept(&self, request: Request<CreateDeptRequest>) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::department::dto::CreateDeptCommand {
            name: req.name, parent_id: req.parent_id, sort: req.sort,
            leader_user_id: req.leader_user_id, phone: req.phone, email: req.email,
        };
        services::get().dept.create_dept(cmd, None).await
            .map(|r| Response::new(map_dept(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_dept(&self, request: Request<UpdateDeptRequest>) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::department::dto::UpdateDeptCommand {
            dept_id: req.dept_id, name: req.name, parent_id: req.parent_id,
            sort: req.sort, leader_user_id: req.leader_user_id,
            phone: req.phone, email: req.email,
        };
        services::get().dept.update_dept(cmd, None).await
            .map(|r| Response::new(map_dept(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_dept(&self, request: Request<DeleteDeptRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().dept.delete_dept(req.dept_id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_dept(&self, request: Request<GetDeptRequest>) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        let query = admin_app::department::dto::DeptQueryRequest { name: None, status: None };
        services::get().dept.get_dept_list(query).await
            .map(|list| {
                let found = list.into_iter().find(|d| d.id == req.dept_id)
                    .expect("dept not found");
                Response::new(map_dept(found))
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_depts(&self, request: Request<ListDeptsRequest>) -> Result<Response<ListDeptsResponse>, Status> {
        let req = request.into_inner();
        let query = admin_app::department::dto::DeptQueryRequest {
            name: req.name, status: req.status,
        };
        services::get().dept.get_dept_list(query).await
            .map(|list| Response::new(ListDeptsResponse {
                items: list.into_iter().map(map_dept).collect(),
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
