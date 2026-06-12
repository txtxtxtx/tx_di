//! 用户管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::user::user_service_server::UserService;
use admin_proto::admin::user::{
    CreateUserRequest, UserResponse, UpdateUserRequest, DeleteUserRequest,
    GetUserRequest, ListUsersRequest, ListUsersResponse,
    ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest,
    ChangeUserStatusRequest,
};
use admin_proto::Empty;
use admin_proto::admin::common::PageResponse;
use admin_domain::user::model::value_object::{Sex, UserStatus};
use crate::services;

#[derive(Debug, Default)]
pub struct UserGrpcService;

#[tonic::async_trait]
impl UserService for UserGrpcService {
    async fn create_user(&self, request: Request<CreateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::user::dto::CreateUserCommand {
            username: req.username, password: req.password, nickname: req.nickname,
            email: req.email, mobile: req.mobile, sex: req.sex.map(Sex::from),
            remark: req.remark,
            role_ids: if req.role_ids.is_empty() { None } else { Some(req.role_ids) },
            dept_ids: if req.dept_ids.is_empty() { None } else { Some(req.dept_ids) },
        };
        services::get().user.create_user(cmd, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_user(&self, request: Request<UpdateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::user::dto::UpdateUserCommand {
            user_id: req.user_id, nickname: req.nickname, email: req.email,
            mobile: req.mobile, sex: req.sex.map(Sex::from),
            status: req.status.and_then(|s| UserStatus::try_from_i32(s).ok()),
            remark: req.remark,
        };
        services::get().user.update_user(cmd, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn change_user_status(&self, request: Request<ChangeUserStatusRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let status = UserStatus::try_from_i32(req.status)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        services::get().user.change_status(req.user_id, status, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_user(&self, request: Request<DeleteUserRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.delete_user(req.user_id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        services::get().user.get_user(req.user_id).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_users(&self, request: Request<ListUsersRequest>) -> Result<Response<ListUsersResponse>, Status> {
        let req = request.into_inner();
        let status = req.status.and_then(|s| UserStatus::try_from_i32(s).ok());
        let page_info = req.page_info.unwrap_or_default();
        let query = admin_app::user::dto::UserQueryRequest {
            username: req.username, nickname: req.nickname, mobile: req.mobile,
            status, dept_id: req.dept_id, page: page_info.page, size: page_info.size,
        };
        services::get().user.get_user_page(query).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                Response::new(ListUsersResponse { items: p.list, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn change_password(&self, request: Request<ChangePasswordRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::user::dto::ChangePasswordCommand { user_id: req.user_id, new_password: req.new_password };
        services::get().user.change_password(cmd, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn assign_roles(&self, request: Request<AssignRolesRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::user::dto::AssignRolesCommand { user_id: req.user_id, role_ids: req.role_ids };
        services::get().user.assign_roles(cmd).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn assign_depts(&self, request: Request<AssignDeptsRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::user::dto::AssignDeptsCommand { user_id: req.user_id, dept_ids: req.dept_ids };
        services::get().user.assign_departments(cmd).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
