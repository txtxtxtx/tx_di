//! 用户角色关联表

use toasty::Model;

/// 用户角色关联实体
#[derive(Debug, Clone, Model)]
#[table = "system_user_role"]
pub struct UserRole {
    #[key]
    #[auto]
    pub id: u64,
    #[index]
    pub user_id: u64,
    #[index]
    pub role_id: u64,
    pub tenant_id: u64,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}
