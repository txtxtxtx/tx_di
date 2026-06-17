use serde::{Deserialize, Serialize};
use admin_domain::shared::model::value_object::TenantId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginCommand {
    pub username: String,
    pub password: String,
    pub login_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LoginResponse {
    pub user_id: u64,
    pub username: String,
    pub nickname: String,
    pub tenant_id: TenantId,
    pub role_ids: Vec<u64>,
    /// 角色编码列表，用于 sa-token 角色校验（如 "super_admin", "user"）
    pub role_codes: Vec<String>,
    pub permissions: Vec<String>,
    pub dept_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UserInfoResponse {
    pub user_id: u64,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub avatar: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutCommand {
    pub user_id: u64,
}
