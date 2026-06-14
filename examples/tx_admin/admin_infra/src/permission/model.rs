use toasty::Model;

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

    #[default(0)]
    pub status: i32,

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
