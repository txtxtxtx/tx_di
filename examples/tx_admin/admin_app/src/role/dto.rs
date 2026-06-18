use admin_domain::role::model::aggregate::Role;

// Re-export proto types directly (no hand-written DTOs)
pub use admin_proto::{CreateRoleRequest, UpdateRoleRequest, AssignMenusRequest, ListRolesRequest, RoleResponse};

/// 将领域层的 Role 聚合根转换为 proto 的 RoleResponse
pub fn role_to_response(role: Role) -> RoleResponse {
    RoleResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        sort: role.sort,
        data_scope: role.data_scope,
        status: role.status,
        remark: role.remark,
        menu_ids: role.menu_ids,
    }
}
