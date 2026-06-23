//! 部门管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::dept::department_service_server::DepartmentService;
use admin_proto::admin::dept::{
    CreateDeptRequest, DeleteDeptRequest, DeptResponse, GetDeptRequest, ListDeptsRequest,
    ListDeptsResponse, UpdateDeptRequest,
};
use admin_proto::Empty;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct DeptGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl DepartmentService for DeptGrpcService {
    async fn create_dept(
        &self,
        request: Request<CreateDeptRequest>,
    ) -> Result<Response<DeptResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dept:create").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::department::app_service::DepartmentAppService> = self.app.inject();
        let r = svc.create_dept(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_dept(
        &self,
        request: Request<UpdateDeptRequest>,
    ) -> Result<Response<DeptResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dept:update").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::department::app_service::DepartmentAppService> = self.app.inject();
        let r = svc.update_dept(req, Some(login_id)).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_dept(
        &self,
        request: Request<DeleteDeptRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dept:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::department::app_service::DepartmentAppService> = self.app.inject();
        svc.delete_dept(req.dept_id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_dept(
        &self,
        request: Request<GetDeptRequest>,
    ) -> Result<Response<DeptResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dept:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::department::app_service::DepartmentAppService> = self.app.inject();
        let query = ListDeptsRequest {
            name: None,
            status: None,
        };
        let list = svc.get_dept_list(query).await.map_err(err::to_status)?;
        let found = list
            .into_iter()
            .find(|d| d.id == req.dept_id)
            .ok_or_else(|| Status::not_found("dept not found"))?;
        Ok(Response::new(found))
    }

    async fn list_depts(
        &self,
        request: Request<ListDeptsRequest>,
    ) -> Result<Response<ListDeptsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "dept:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::department::app_service::DepartmentAppService> = self.app.inject();
        let list = svc.get_dept_list(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListDeptsResponse { items: list }))
    }
}
