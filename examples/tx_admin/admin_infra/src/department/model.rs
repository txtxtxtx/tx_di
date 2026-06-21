use toasty::Model;

use crate::common::{Status, Deleted};

/// 系统部门表
#[derive(Debug, Clone, Model)]
#[table = "sys_department"]
pub struct SysDepartment {
    #[key]
    #[auto]
    pub id: i64,

    #[default("".to_string())]
    pub name: String,

    #[default(0)]
    pub parent_id: i64,

    #[default(0)]
    pub sort: i32,

    #[default(0)]
    pub leader_user_id: i64,

    #[default("".to_string())]
    pub phone: String,

    #[default("".to_string())]
    pub email: String,

    #[default(Status::Disabled)]
    pub status: Status,

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
