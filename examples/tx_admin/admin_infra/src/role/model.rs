use toasty::Model;

use crate::common::{Status, Deleted};

/// 系统角色表
#[derive(Debug, Clone, Model)]
#[table = "sys_role"]
pub struct SysRole {
    #[key]
    #[auto]
    pub id: i64,

    #[default("".to_string())]
    pub name: String,

    #[unique]
    pub code: String,

    #[default(0)]
    pub sort: i32,

    #[default(4)]
    pub data_scope: i32,

    #[default("".to_string())]
    pub data_scope_dept_ids: String,

    #[default(Status::Disabled)]
    pub status: Status,

    #[default("".to_string())]
    pub remark: String,

    #[default(0)]
    pub tenant_id: i32,

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

/// 角色-菜单关联表
#[derive(Debug, Clone, Model)]
#[table = "sys_role_menu"]
pub struct SysRoleMenu {
    #[key]
    #[auto]
    pub id: i64,

    #[index]
    pub role_id: i64,

    #[index]
    pub menu_id: i64,

    #[auto]
    pub created_at: jiff::Timestamp,
}
