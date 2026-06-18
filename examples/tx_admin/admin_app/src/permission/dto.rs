use admin_domain::permission::model::aggregate::Permission;
use admin_domain::permission::model::value_object::PermissionCheck;
use admin_proto::UserPermissionItem;
use admin_proto::PermissionDetail;

/// Convert a domain PermissionCheck into a proto UserPermissionItem
pub fn permission_check_to_item(pc: PermissionCheck) -> UserPermissionItem {
    UserPermissionItem {
        code: pc.code,
        name: pc.name,
        permission_type: format!("{:?}", pc.permission_type),
    }
}

/// Convert a domain Permission aggregate into a proto PermissionDetail
pub fn permission_to_detail(p: Permission) -> PermissionDetail {
    PermissionDetail {
        id: p.id,
        name: p.name,
        permission_code: p.permission_code,
        r#type: p.permission_type as i32,
        parent_id: p.parent_id,
        sort: p.sort,
        description: p.description.unwrap_or_default(),
        status: p.status,
    }
}
