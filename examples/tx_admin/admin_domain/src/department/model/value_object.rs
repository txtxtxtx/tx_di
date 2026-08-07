use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeptQuery {
    pub name: Option<String>,
    pub status: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeptTreeNode {
    pub id: u64,
    pub name: String,
    pub parent_id: u64,
    pub sort: i32,
    pub leader_user_id: Option<u64>,
    pub status: i32,
    pub children: Vec<DeptTreeNode>,
}
