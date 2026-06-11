//! 部门管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::dept::department_service_server::DepartmentService;
use admin_proto::admin::dept::{
    CreateDeptRequest, DeptResponse, UpdateDeptRequest, DeleteDeptRequest,
    GetDeptRequest, ListDeptsRequest, ListDeptsResponse,
};
use admin_proto::Empty;

/// 部门 gRPC 服务
#[derive(Debug, Default)]
pub struct DeptGrpcService;

#[tonic::async_trait]
impl DepartmentService for DeptGrpcService {
    async fn create_dept(
        &self,
        request: Request<CreateDeptRequest>,
    ) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DeptAppService::create
        let resp = DeptResponse {
            id: 1,
            name: req.name.clone(),
            parent_id: req.parent_id,
            sort: req.sort,
            leader_user_id: req.leader_user_id,
            phone: req.phone.clone(),
            email: req.email.clone(),
            status: 1,
        };
        Ok(Response::new(resp))
    }

    async fn update_dept(
        &self,
        request: Request<UpdateDeptRequest>,
    ) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DeptAppService::update
        let resp = DeptResponse {
            id: req.dept_id,
            name: req.name.clone(),
            parent_id: req.parent_id,
            sort: req.sort,
            leader_user_id: req.leader_user_id,
            phone: req.phone.clone(),
            email: req.email.clone(),
            status: 1,
        };
        Ok(Response::new(resp))
    }

    async fn delete_dept(
        &self,
        request: Request<DeleteDeptRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DeptAppService::delete
        let _ = req.dept_id;
        Ok(Response::new(Empty {}))
    }

    async fn get_dept(
        &self,
        request: Request<GetDeptRequest>,
    ) -> Result<Response<DeptResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 DeptAppService::get_by_id
        let resp = DeptResponse {
            id: req.dept_id,
            name: "placeholder".into(),
            parent_id: 0,
            sort: 0,
            leader_user_id: None,
            phone: None,
            email: None,
            status: 1,
        };
        Ok(Response::new(resp))
    }

    async fn list_depts(
        &self,
        _request: Request<ListDeptsRequest>,
    ) -> Result<Response<ListDeptsResponse>, Status> {
        // TODO: 调用 DeptAppService::list
        let resp = ListDeptsResponse { items: vec![] };
        Ok(Response::new(resp))
    }
}
