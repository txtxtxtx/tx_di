//! 权限管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::permission::permission_service_server::PermissionService;
use admin_proto::admin::permission::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse,
};

/// 权限 gRPC 服务
#[derive(Debug, Default)]
pub struct PermissionGrpcService;

#[tonic::async_trait]
impl PermissionService for PermissionGrpcService {
    async fn check_permission(
        &self,
        _request: Request<PermissionCheckRequest>,
    ) -> Result<Response<PermissionCheckResponse>, Status> {
        // TODO: 调用 PermissionAppService::check
        let resp = PermissionCheckResponse { has_permission: true };
        Ok(Response::new(resp))
    }

    async fn get_user_permissions(
        &self,
        _request: Request<GetUserPermissionsRequest>,
    ) -> Result<Response<UserPermissionsResponse>, Status> {
        // TODO: 调用 PermissionAppService::get_user_permissions
        let resp = UserPermissionsResponse {
            user_id: 0,
            permissions: vec![],
            items: vec![],
        };
        Ok(Response::new(resp))
    }
}
