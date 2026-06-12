//! 权限管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::permission::permission_service_server::PermissionService;
use admin_proto::admin::permission::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse,
};

#[derive(Debug, Default)]
pub struct PermissionGrpcService;

#[tonic::async_trait]
impl PermissionService for PermissionGrpcService {
    async fn check_permission(&self, request: Request<PermissionCheckRequest>) -> Result<Response<PermissionCheckResponse>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::permission::dto::PermissionCheckRequest {
            user_id: req.user_id, permission: req.permission,
        };
        services::get().perm.check_permission(cmd).await
            .map(|r| Response::new(PermissionCheckResponse { has_permission: r.has_permission }))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_user_permissions(&self, request: Request<GetUserPermissionsRequest>) -> Result<Response<UserPermissionsResponse>, Status> {
        let req = request.into_inner();
        services::get().perm.get_user_permissions(req.user_id).await
            .map(|r| Response::new(UserPermissionsResponse {
                user_id: r.user_id, permissions: r.permissions, items: vec![],
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
