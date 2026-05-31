//! 用户 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: u64,
    pub tenant_id: u64,
    pub username: String,
    pub nickname: String,
    pub email: String,
    pub mobile: String,
    pub sex: String,
    pub avatar: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&crate::domain::user::User> for UserDto {
    fn from(u: &crate::domain::user::User) -> Self {
        Self {
            id: u.id, tenant_id: u.tenant_id, username: u.username.clone(), nickname: u.nickname.clone(),
            email: u.email.clone(), mobile: u.mobile.clone(), sex: u.sex.to_string(), avatar: u.avatar.clone(),
            status: u.status.to_string(),
            created_at: u.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: u.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
}
