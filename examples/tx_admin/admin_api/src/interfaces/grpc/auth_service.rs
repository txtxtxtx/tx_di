//! 认证 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::auth::auth_service_server::AuthService;
use admin_proto::admin::auth::{
    LoginRequest, LoginResponse, GetUserInfoRequest, UserInfoResponse, LogoutRequest,
};
use admin_proto::Empty;
use crate::services;

#[derive(Debug, Default)]
pub struct AuthGrpcService;

#[tonic::async_trait]
impl AuthService for AuthGrpcService {
    async fn login(&self, request: Request<LoginRequest>) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::auth::dto::LoginCommand {
            username: req.username,
            password: req.password,
            login_ip: req.login_ip,
        };
        services::get().auth.login(cmd).await
            .map(|r| Response::new(LoginResponse {
                user_id: r.user_id, username: r.username, nickname: r.nickname,
                tenant_id: r.tenant_id.into_inner() as i64,
                role_ids: r.role_ids, permissions: r.permissions, dept_ids: r.dept_ids,
                token: String::new(),
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_user_info(&self, request: Request<GetUserInfoRequest>) -> Result<Response<UserInfoResponse>, Status> {
        let req = request.into_inner();
        services::get().auth.get_user_info(req.user_id).await
            .map(|r| Response::new(UserInfoResponse {
                user_id: r.user_id, username: r.username, nickname: r.nickname,
                email: r.email, mobile: r.mobile, avatar: r.avatar,
                roles: r.roles, permissions: r.permissions, tenant_id: 0,
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn logout(&self, request: Request<LogoutRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::auth::dto::LogoutCommand { user_id: req.user_id };
        services::get().auth.logout(cmd).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
