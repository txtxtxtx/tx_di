use serde::{Deserialize, Serialize};
use admin_domain::permission::model::aggregate::Permission;
use admin_domain::permission::model::value_object::PermissionCheck;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckRequest {
    pub user_id: u64,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResponse {
    pub has_permission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissionsResponse {
    pub user_id: u64,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionItem {
    pub code: String,
    pub name: String,
    pub permission_type: String,
}

impl From<PermissionCheck> for PermissionItem {
    fn from(pc: PermissionCheck) -> Self {
        Self {
            code: pc.code,
            name: pc.name,
            permission_type: format!("{:?}", pc.permission_type),
        }
    }
}

// === 新增 CRUD DTOs ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePermissionCommand {
    pub name: String,
    pub permission_code: String,
    pub permission_type: i32,
    pub parent_id: u64,
    pub sort: i32,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePermissionCommand {
    pub id: u64,
    pub name: String,
    pub permission_code: String,
    pub permission_type: i32,
    pub parent_id: u64,
    pub sort: i32,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub description: Option<String>,
}

/// Permission CRUD response (full entity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub id: u64,
    pub name: String,
    pub permission_code: String,
    pub permission_type: i32,
    pub parent_id: u64,
    pub sort: i32,
    pub description: Option<String>,
    pub status: i32,
}

impl From<Permission> for PermissionResponse {
    fn from(p: Permission) -> Self {
        Self {
            id: p.id,
            name: p.name,
            permission_code: p.permission_code,
            permission_type: p.permission_type as i32,
            parent_id: p.parent_id,
            sort: p.sort,
            description: p.description,
            status: p.status,
        }
    }
}
