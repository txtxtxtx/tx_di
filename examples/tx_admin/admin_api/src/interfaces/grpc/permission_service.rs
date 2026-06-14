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

fn map_permission_detail(r: admin_app::permission::dto::PermissionResponse) -> PermissionDetail {
    PermissionDetail {
        id: r.id,
        name: r.name,
        permission_code: r.permission_code,
        r#type: r.permission_type,
        parent_id: r.parent_id,
        sort: r.sort,
        description: r.description.unwrap_or_default(),
        status: r.status,
    }
}

#[tonic::async_trait]
impl ProtoPermissionService for PermissionGrpcService {
    // === 原有查询方法 ===

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

    // === 新增 CRUD 方法 ===

    async fn create_permission(&self, request: Request<CreatePermissionRequest>) -> Result<Response<PermissionDetail>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::permission::dto::CreatePermissionCommand {
            name: req.name,
            permission_code: req.permission_code,
            permission_type: req.r#type,
            parent_id: req.parent_id,
            sort: req.sort,
            description: if req.description.is_empty() { None } else { Some(req.description) },
        };
        services::get().perm.create_permission(cmd, None).await
            .map(|r| Response::new(map_permission_detail(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn update_permission(&self, request: Request<UpdatePermissionRequest>) -> Result<Response<PermissionDetail>, Status> {
        let req = request.into_inner();
        let cmd = admin_app::permission::dto::UpdatePermissionCommand {
            id: req.id,
            name: req.name,
            permission_code: req.permission_code,
            permission_type: req.r#type,
            parent_id: req.parent_id,
            sort: req.sort,
            description: if req.description.is_empty() { None } else { Some(req.description) },
        };
        services::get().perm.update_permission(cmd, None).await
            .map(|r| Response::new(map_permission_detail(r)))
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
            .map(|r| Response::new(map_permission_detail(r)))
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn list_permissions(&self, _request: Request<ListPermissionsRequest>) -> Result<Response<ListPermissionsResponse>, Status> {
        services::get().perm.get_permission_list().await
            .map(|list| Response::new(ListPermissionsResponse {
                permissions: list.into_iter().map(map_permission_detail).collect(),
            }))
            .map_err(|e| Status::internal(e.to_string()))
    }
}
