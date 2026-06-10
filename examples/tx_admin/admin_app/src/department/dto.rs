use serde::{Deserialize, Serialize};
use admin_domain::department::model::value_object::DeptTreeNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeptCommand {
    pub name: String,
    pub parent_id: u64,
    pub sort: i32,
    pub leader_user_id: Option<u64>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDeptCommand {
    pub dept_id: u64,
    pub name: String,
    pub parent_id: u64,
    pub sort: i32,
    pub leader_user_id: Option<u64>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeptQueryRequest {
    pub name: Option<String>,
    pub status: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeptResponse {
    pub id: u64,
    pub name: String,
    pub parent_id: u64,
    pub sort: i32,
    pub leader_user_id: Option<u64>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: i32,
}

impl From<admin_domain::department::model::aggregate::Department> for DeptResponse {
    fn from(dept: admin_domain::department::model::aggregate::Department) -> Self {
        Self {
            id: dept.id,
            name: dept.name,
            parent_id: dept.parent_id,
            sort: dept.sort,
            leader_user_id: dept.leader_user_id,
            phone: dept.phone,
            email: dept.email,
            status: dept.status,
        }
    }
}
