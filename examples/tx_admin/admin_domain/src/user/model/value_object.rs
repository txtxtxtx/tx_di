use serde::{Deserialize, Serialize};

/// User query filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserQuery {
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub status: Option<i32>,
    pub dept_id: Option<u64>,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
}

/// User info for display (without password)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDisplayInfo {
    pub id: u64,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: i32,
    pub avatar: Option<String>,
    pub status: i32,
    pub dept_names: Vec<String>,
    pub role_names: Vec<String>,
}

/// User login info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginUser {
    pub user_id: u64,
    pub username: String,
    pub nickname: String,
    pub tenant_id: i32,
    pub role_ids: Vec<u64>,
    pub permissions: Vec<String>,
    pub dept_ids: Vec<u64>,
}
