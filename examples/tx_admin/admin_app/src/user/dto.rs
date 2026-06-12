use serde::{Deserialize, Serialize};
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{Sex, UserStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserCommand {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: Option<Sex>,
    pub remark: Option<String>,
    pub role_ids: Option<Vec<u64>>,
    pub dept_ids: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserCommand {
    pub user_id: u64,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: Option<Sex>,
    pub status: Option<UserStatus>,
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
    pub status: Option<UserStatus>,
    pub dept_id: Option<u64>,
    pub page: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: u64,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub sex: Sex,
    pub status: UserStatus,
    pub remark: Option<String>,
    pub role_ids: Vec<u64>,
    pub dept_ids: Vec<u64>,
    pub avatar: Option<String>,
    pub login_ip: Option<String>,
    pub login_date: Option<i64>,
    pub tenant_id: u64,
    pub create_time: i64,
    pub update_time: i64,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
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
            avatar: user.avatar,
            login_ip: user.login_ip,
            login_date: user.login_date.map(|d| d.timestamp_millis()),
            tenant_id: user.tenant_id.into_inner(),
            create_time: user.audit.create_time.timestamp_millis(),
            update_time: user.audit.update_time.timestamp_millis(),
        }
    }
}
