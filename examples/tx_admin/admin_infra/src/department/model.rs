use toasty::Model;

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

    #[default(0)]
    pub status: i32,

    #[default(0)]
    pub tenant_id: i32,

    #[default("".to_string())]
    pub creator: String,

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(0)]
    pub deleted: i32,
}
