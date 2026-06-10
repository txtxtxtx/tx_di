use serde::{Deserialize, Serialize};
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

impl From<PermissionCheck> for PermissionItem {
    fn from(pc: PermissionCheck) -> Self {
        Self {
            code: pc.code,
            name: pc.name,
            permission_type: format!("{:?}", pc.permission_type),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionItem {
    pub code: String,
    pub name: String,
    pub permission_type: String,
}
