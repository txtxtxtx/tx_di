use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserCommand {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: Option<i32>,
    pub remark: Option<String>,
    pub role_ids: Option<Vec<u64>>,
    pub dept_ids: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserCommand {
    pub user_id: u64,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: i32,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordCommand {
    pub user_id: u64,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRolesCommand {
    pub user_id: u64,
    pub role_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignDeptsCommand {
    pub user_id: u64,
    pub dept_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQueryRequest {
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub status: Option<i32>,
    pub dept_id: Option<u64>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: u64,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: i32,
    pub status: i32,
    pub remark: Option<String>,
    pub role_ids: Vec<u64>,
    pub dept_ids: Vec<u64>,
}

impl From<admin_domain::user::model::aggregate::User> for UserResponse {
    fn from(user: admin_domain::user::model::aggregate::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            nickname: user.nickname,
            email: user.email,
            mobile: user.mobile,
            sex: user.sex,
            status: user.status,
            remark: user.remark,
            role_ids: user.role_ids,
            dept_ids: user.dept_ids,
        }
    }
}
