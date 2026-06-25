use toasty::Model;

use crate::common::{Sex, Status, Deleted};

/// 系统用户表
#[derive(Debug, Clone, Model)]
#[table = "sys_user"]
pub struct SysUser {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub username: String,

    pub password_hash: String,

    #[default("".to_string())]
    pub nickname: String,

    #[default("".to_string())]
    pub remark: String,

    #[default("".to_string())]
    pub email: String,

    #[default("".to_string())]
    pub mobile: String,

    #[default(Sex::Unknown)]
    pub sex: Sex,

    #[default("".to_string())]
    pub avatar: String,

    #[default(Status::Disabled)]
    pub status: Status,

    #[default("".to_string())]
    pub login_ip: String,

    #[default(jiff::Timestamp::UNIX_EPOCH)]
    pub login_date: jiff::Timestamp,

    #[default(0)]
    pub tenant_id: u64,

    #[default("".to_string())]
    pub creator: String,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[default("".to_string())]
    pub updater: String,

    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}

/// 用户-角色关联表
#[derive(Debug, Clone, Model)]
#[table = "sys_user_role"]
pub struct SysUserRole {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    #[index]
    pub role_id: u64,

    #[auto]
    pub created_at: jiff::Timestamp,
}

/// 用户-部门关联表
#[derive(Debug, Clone, Model)]
#[table = "sys_user_dept"]
pub struct SysUserDept {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    #[index]
    pub dept_id: u64,

    #[auto]
    pub created_at: jiff::Timestamp,
}
