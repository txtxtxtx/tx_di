//! 部门管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::dept::department_service_server::DepartmentService;
use admin_proto::admin::dept::{
    CreateDeptRequest, DeptResponse, UpdateDeptRequest, DeleteDeptRequest,
    GetDeptRequest, ListDeptsRequest, ListDeptsResponse,
};
use admin_proto::Empty;

#[derive(Debug, Default)]
pub struct DeptGrpcService;

#[tonic::async_trait]
impl DepartmentService for DeptGrpcService {
    async fn create_dept(&self, request: Request<CreateDeptRequest>) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        services::get().dept.create_dept(req, None).await
            .map(|r| Response::new(r))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_dept(&self, request: Request<UpdateDeptRequest>) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        services::get().dept.update_dept(req, None).await
            .map(|r| Response::new(r))
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
        let query = ListDeptsRequest { name: None, status: None };
        services::get().dept.get_dept_list(query).await
            .map(|list| {
                let found = list.into_iter().find(|d| d.id == req.dept_id)
                    .expect("dept not found");
                Response::new(found)
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_depts(&self, request: Request<ListDeptsRequest>) -> Result<Response<ListDeptsResponse>, Status> {
        let req = request.into_inner();
        services::get().dept.get_dept_list(req).await
            .map(|list| Response::new(ListDeptsResponse {
                items: list,
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
