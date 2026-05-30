//! 认证 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: u64,
    pub username: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub tenant_id: u64,
}
