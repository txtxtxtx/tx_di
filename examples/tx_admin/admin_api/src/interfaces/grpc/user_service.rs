//! 用户管理 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};

use admin_proto::admin::user::user_service_server::UserService;
use admin_proto::admin::user::{
    AssignDeptsRequest, AssignRolesRequest, ChangePasswordRequest, ChangeUserStatusRequest,
    CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, ListUsersResponse,
    UpdateUserRequest, UserIdRequest, UserResponse,
};
use admin_proto::admin::common::PageResponse;
use admin_proto::Empty;
use admin_domain::user::model::value_object::UserStatus;
use tx_di_core::App;

use super::auth_interceptor::{self, get_login_id};
use super::err;

#[derive(Clone)]
pub struct UserGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl UserService for UserGrpcService {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:create").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        let r = svc
            .create_user(req, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let login_id = get_login_id(&request)?;
        let req = request.into_inner();

        // 自己编辑自己不需要权限
        if login_id != req.user_id.to_string() {
            auth_interceptor::ensure_grpc_permission(&login_id, "user:update").await?;
        }

        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        let r = svc
            .update_user(req, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:delete").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.delete_user(req.user_id, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        let r = svc.get_user(req.user_id).await.map_err(err::to_status)?;
        Ok(Response::new(r))
    }

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:view").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        let p = svc.get_user_page(req).await.map_err(err::to_status)?;
        Ok(Response::new(ListUsersResponse {
            items: p.list,
            page_info: Some(PageResponse {
                total: p.total,
                page: p.page,
                size: p.size,
            }),
        }))
    }

    async fn change_password(
        &self,
        request: Request<ChangePasswordRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        let req = request.into_inner();

        // 自己改自己不需要权限
        if req.user_id.to_string() != login_id {
            auth_interceptor::ensure_grpc_permission(&login_id, "user:password").await?;
        }

        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.change_password(req, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn assign_roles(
        &self,
        request: Request<AssignRolesRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:assign_role").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.assign_roles(req.user_id, req.role_ids).await.map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn assign_depts(
        &self,
        request: Request<AssignDeptsRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:assign_dept").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.assign_departments(req.user_id, req.dept_ids)
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn change_user_status(
        &self,
        request: Request<ChangeUserStatusRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:status").await?;

        let req = request.into_inner();
        let status = UserStatus::try_from_i32(req.status)
            .map_err(|_| Status::invalid_argument("invalid status"))?;
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.change_status(req.user_id, status, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn enable_user(
        &self,
        request: Request<UserIdRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:status").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.change_status(req.user_id, UserStatus::Active, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn disable_user(
        &self,
        request: Request<UserIdRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:status").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.change_status(req.user_id, UserStatus::Disabled, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn lock_user(
        &self,
        request: Request<UserIdRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:status").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.change_status(req.user_id, UserStatus::Locked, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }

    async fn unlock_user(
        &self,
        request: Request<UserIdRequest>,
    ) -> Result<Response<Empty>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "user:status").await?;

        let req = request.into_inner();
        let svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();
        svc.change_status(req.user_id, UserStatus::Active, Some(login_id))
            .await
            .map_err(err::to_status)?;
        Ok(Response::new(Empty {}))
    }
}
