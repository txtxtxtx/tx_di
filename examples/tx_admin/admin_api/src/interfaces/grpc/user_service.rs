//! 用户管理 gRPC 服务实现
//!
//! 实现 tonic 生成的 UserService trait，
//! 使用与 HTTP 相同的 proto DTO。

use tonic::{Request, Response, Status};

use admin_proto::admin::user::user_service_server::UserService;
use admin_proto::admin::user::{
    CreateUserRequest, UserResponse, UpdateUserRequest, DeleteUserRequest,
    GetUserRequest, ListUsersRequest, ListUsersResponse,
    ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest,
};
use admin_proto::Empty;

/// 用户 gRPC 服务
#[derive(Debug, Default)]
pub struct UserGrpcService;

#[tonic::async_trait]
impl UserService for UserGrpcService {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 UserAppService::create
        let resp = UserResponse {
            id: 1,
            username: req.username.clone(),
            nickname: req.nickname.clone(),
            email: req.email.clone(),
            mobile: req.mobile.clone(),
            sex: req.sex.unwrap_or(0),
            status: 1,
            remark: req.remark.clone(),
            role_ids: req.role_ids.clone(),
            dept_ids: req.dept_ids.clone(),
        };
        Ok(Response::new(resp))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 UserAppService::update
        let resp = UserResponse {
            id: req.user_id,
            username: String::new(),
            nickname: req.nickname.clone(),
            email: req.email.clone(),
            mobile: req.mobile.clone(),
            sex: req.sex,
            status: 1,
            remark: req.remark.clone(),
            role_ids: vec![],
            dept_ids: vec![],
        };
        Ok(Response::new(resp))
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // TODO: 调用 UserAppService::delete
        let _ = req.user_id;
        Ok(Response::new(Empty {}))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 UserAppService::get_by_id
        let resp = UserResponse {
            id: req.user_id,
            username: "placeholder".into(),
            nickname: "Placeholder".into(),
            email: None,
            mobile: None,
            sex: 0,
            status: 1,
            remark: None,
            role_ids: vec![],
            dept_ids: vec![],
        };
        Ok(Response::new(resp))
    }

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 调用 UserAppService::list
        let resp = ListUsersResponse {
            items: vec![],
            page_info: None,
        };
        Ok(Response::new(resp))
    }

    async fn change_password(
        &self,
        _request: Request<ChangePasswordRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 UserAppService::change_password
        Ok(Response::new(Empty {}))
    }

    async fn assign_roles(
        &self,
        _request: Request<AssignRolesRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 UserAppService::assign_roles
        Ok(Response::new(Empty {}))
    }

    async fn assign_depts(
        &self,
        _request: Request<AssignDeptsRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 UserAppService::assign_depts
        Ok(Response::new(Empty {}))
    }
}
