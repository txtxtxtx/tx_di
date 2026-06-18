//! 权限管理 gRPC 服务实现

use tonic::{Request, Response, Status};

use admin_proto::admin::permission::permission_service_server::PermissionService as ProtoPermissionService;
use admin_proto::admin::permission::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse,
    CreatePermissionRequest, UpdatePermissionRequest, DeletePermissionRequest,
    GetPermissionRequest, ListPermissionsRequest, ListPermissionsResponse,
    PermissionDetail,
};
use admin_proto::Empty;

#[derive(Debug, Default)]
pub struct PermissionGrpcService;

#[tonic::async_trait]
impl ProtoPermissionService for PermissionGrpcService {
    // === 原有查询方法 ===

    async fn check_permission(&self, request: Request<PermissionCheckRequest>) -> Result<Response<PermissionCheckResponse>, Status> {
        let req = request.into_inner();
        services::get().perm.check_permission(req).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_user_permissions(&self, request: Request<GetUserPermissionsRequest>) -> Result<Response<UserPermissionsResponse>, Status> {
        let req = request.into_inner();
        services::get().perm.get_user_permissions(req.user_id).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    // === 新增 CRUD 方法 ===

    async fn create_permission(&self, request: Request<CreatePermissionRequest>) -> Result<Response<PermissionDetail>, Status> {
        let req = request.into_inner();
        services::get().perm.create_permission(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_permission(&self, request: Request<UpdatePermissionRequest>) -> Result<Response<PermissionDetail>, Status> {
        let req = request.into_inner();
        services::get().perm.update_permission(req, None).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn delete_permission(&self, request: Request<DeletePermissionRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        services::get().perm.delete_permission(req.id, None).await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_permission(&self, request: Request<GetPermissionRequest>) -> Result<Response<PermissionDetail>, Status> {
        let req = request.into_inner();
        services::get().perm.get_permission(req.id).await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_permissions(&self, _request: Request<ListPermissionsRequest>) -> Result<Response<ListPermissionsResponse>, Status> {
        services::get().perm.get_permission_list().await
            .map(|list| Response::new(ListPermissionsResponse {
                permissions: list,
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
