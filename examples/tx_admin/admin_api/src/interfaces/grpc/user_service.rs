//! 用户管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::user::user_service_server::UserService;
use admin_proto::admin::user::{
    CreateUserRequest, UserResponse, UpdateUserRequest, DeleteUserRequest,
    GetUserRequest, ListUsersRequest, ListUsersResponse,
    ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest,
    ChangeUserStatusRequest, UserIdRequest,
};
use admin_proto::Empty;
use admin_proto::admin::common::PageResponse;
use admin_domain::user::model::value_object::UserStatus;

#[derive(Debug, Default)]
pub struct UserGrpcService;

#[tonic::async_trait]
impl UserService for UserGrpcService {
    async fn create_user(&self, request: Request<CreateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        services::get().user.create_user(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_user(&self, request: Request<UpdateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        services::get().user.update_user(req, None).await
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
        services::get().user.get_user_page(req).await
            .map(|p| {
                let total = p.total; let page = p.page; let size = p.size;
                Response::new(ListUsersResponse { items: p.list, page_info: Some(PageResponse { total, page, size }) })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn change_password(&self, request: Request<ChangePasswordRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.change_password(req, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn assign_roles(&self, request: Request<AssignRolesRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.assign_roles(req).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn assign_depts(&self, request: Request<AssignDeptsRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.assign_departments(req).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn enable_user(&self, request: Request<UserIdRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.change_status(req.user_id, UserStatus::Active, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn disable_user(&self, request: Request<UserIdRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.change_status(req.user_id, UserStatus::Disabled, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn lock_user(&self, request: Request<UserIdRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.change_status(req.user_id, UserStatus::Locked, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn unlock_user(&self, request: Request<UserIdRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().user.change_status(req.user_id, UserStatus::Active, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
