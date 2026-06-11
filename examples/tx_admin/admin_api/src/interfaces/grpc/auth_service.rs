//! 认证 gRPC 服务实现
//!
//! 实现 tonic 生成的 AuthService trait，
//! 使用与 HTTP 相同的 proto DTO。

use tonic::{Request, Response, Status};

use admin_proto::admin::auth::auth_service_server::AuthService;
use admin_proto::admin::auth::{
    LoginRequest, LoginResponse,
    GetUserInfoRequest, UserInfoResponse,
    LogoutRequest,
};
use admin_proto::Empty;

/// 认证 gRPC 服务
#[derive(Debug, Default)]
pub struct AuthGrpcService;

#[tonic::async_trait]
impl AuthService for AuthGrpcService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 AuthAppService::login
        let resp = LoginResponse {
            user_id: 1,
            username: req.username.clone(),
            nickname: "Admin".into(),
            tenant_id: 0,
            role_ids: vec![],
            permissions: vec![],
            dept_ids: vec![],
            token: "placeholder-token".into(),
        };
        Ok(Response::new(resp))
    }

    async fn get_user_info(
        &self,
        request: Request<GetUserInfoRequest>,
    ) -> Result<Response<UserInfoResponse>, Status> {
        let req = request.into_inner();
        // TODO: 调用 AuthAppService::get_user_info
        let resp = UserInfoResponse {
            user_id: req.user_id,
            username: "admin".into(),
            nickname: "Admin".into(),
            email: Some("admin@example.com".into()),
            mobile: None,
            avatar: None,
            roles: vec!["admin".into()],
            permissions: vec!["*:*:*".into()],
            tenant_id: 0,
        };
        Ok(Response::new(resp))
    }

    async fn logout(
        &self,
        _request: Request<LogoutRequest>,
    ) -> Result<Response<Empty>, Status> {
        // TODO: 调用 AuthAppService::logout
        Ok(Response::new(Empty {}))
    }
}
