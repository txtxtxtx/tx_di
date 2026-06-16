use toasty::Model;

use crate::common::{Status, Deleted};

/// 系统权限表
#[derive(Debug, Clone, Model)]
#[table = "sys_permission"]
pub struct SysPermission {
    #[key]
    #[auto]
    pub id: i64,

    #[default("".to_string())]
    pub name: String,

    #[unique]
    pub permission_code: String,

    #[default(0)]
    pub permission_type: i32,

    #[default(0)]
    pub parent_id: i64,

    #[default(0)]
    pub sort: i32,

    #[default("".to_string())]
    pub description: String,

    #[default(Status::Disabled)]
    pub status: Status,

    #[default("".to_string())]
    pub creator: String,

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}
