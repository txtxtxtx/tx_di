use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleCommand {
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub menu_ids: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleCommand {
    pub role_id: u64,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub data_scope: i32,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignMenusCommand {
    pub role_id: u64,
    pub menu_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleQueryRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub status: Option<i32>,
    pub page: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: u64,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub data_scope: i32,
    pub status: i32,
    pub remark: Option<String>,
    pub menu_ids: Vec<u64>,
}

impl From<admin_domain::role::model::aggregate::Role> for RoleResponse {
    fn from(role: admin_domain::role::model::aggregate::Role) -> Self {
        Self {
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
}
