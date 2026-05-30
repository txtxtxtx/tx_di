//! 角色 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct RoleDto {
    pub id: u64, pub tenant_id: u64, pub name: String, pub code: String, pub sort: i32,
    pub data_scope: String, pub status: String, pub role_type: String,
    pub remark: Option<String>, pub created_at: String, pub updated_at: String,
}

impl From<&crate::domain::role::Role> for RoleDto {
    fn from(r: &crate::domain::role::Role) -> Self {
        Self { id: r.id, tenant_id: r.tenant_id, name: r.name.clone(), code: r.code.clone(), sort: r.sort,
            data_scope: r.data_scope.to_string(), status: r.status.to_string(), role_type: r.role_type.to_string(),
            remark: r.remark.clone(),
            created_at: r.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: r.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string() }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest { pub name: String, pub code: String, pub remark: Option<String>, pub sort: Option<i32> }

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest { pub name: Option<String>, pub remark: Option<String>, pub sort: Option<i32> }
