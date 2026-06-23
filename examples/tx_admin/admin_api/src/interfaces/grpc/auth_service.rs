//! 认证 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::auth::auth_service_server::AuthService;
use admin_proto::admin::auth::{
    GetUserInfoRequest, LoginRequest, LoginResponse, LogoutRequest, UserInfoResponse,
};
use admin_proto::Empty;
use admin_domain::shared::model::value_object::SessionEctData;
use tx_di_core::App;
use tx_di_sa_token::StpUtil;

use super::err;

const ADMIN_ROLE: &str = "admin";

#[derive(Clone)]
pub struct AuthGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl AuthService for AuthGrpcService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        let login_ip = req.login_ip.clone();
        let auth_svc: Arc<admin_app::auth::app_service::AuthAppService> = self.app.inject();

        let mut r = auth_svc.login(req).await.map_err(err::to_status)?;

        // 通过 sa-token 创建会话
        let token = StpUtil::login(r.user_id.to_string())
            .await
            .map_err(|e| Status::internal(format!("创建会话失败: {}", e)))?;

        // 将登录信息存入 token extra_data
        let extra = serde_json::json!(SessionEctData {
            login_ip,
            tenant_id: r.tenant_id.into(),
            role_ids: r.role_ids.clone(),
            dept_ids: r.dept_ids.clone(),
            username: r.username.clone(),
        });
        let _ = StpUtil::set_extra_data(&token, extra).await;

        // 设置权限和角色
        let user_id_str = r.user_id.to_string();
        let is_admin = r.role_codes.iter().any(|c| c == ADMIN_ROLE);
        if !is_admin {
            let _ = StpUtil::set_permissions(&user_id_str, r.permissions.clone()).await;
        }
        let _ = StpUtil::set_roles(&user_id_str, r.role_codes.clone()).await;

        r.token = token.to_string();
        Ok(Response::new(r))
    }

    async fn get_user_info(
        &self,
        request: Request<GetUserInfoRequest>,
    ) -> Result<Response<UserInfoResponse>, Status> {
        let req = request.into_inner();
        let auth_svc: Arc<admin_app::auth::app_service::AuthAppService> = self.app.inject();

        let r = auth_svc
            .get_user_info(req.user_id)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let auth_svc: Arc<admin_app::auth::app_service::AuthAppService> = self.app.inject();

        // 使当前会话失效
        let _ = StpUtil::logout_current().await;
        // 清除权限和角色缓存
        let user_id_str = req.user_id.to_string();
        let _ = StpUtil::clear_permissions(&user_id_str).await;
        let _ = StpUtil::clear_roles(&user_id_str).await;
        // 记录登出日志
        let _ = auth_svc.logout(req).await;

        Ok(Response::new(Empty {}))
    }
}
