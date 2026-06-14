use toasty::Model;

/// 系统菜单表
#[derive(Debug, Clone, Model)]
#[table = "sys_menu"]
pub struct SysMenu {
    #[key]
    #[auto]
    pub id: i64,

    #[default("".to_string())]
    pub name: String,

    #[default("".to_string())]
    pub permission: String,

    #[default(0)]
    pub types: i32,

    #[default(0)]
    pub sort: i32,

    #[default(0)]
    pub parent_id: i64,

    #[default("".to_string())]
    pub route_path: String,

    #[default("".to_string())]
    pub icon: String,

    #[default("".to_string())]
    pub component: String,

    #[default("".to_string())]
    pub component_name: String,

    #[default(0)]
    pub status: i32,

    #[default(0)]
    pub visible: i32,

    #[default(0)]
    pub keep_alive: i32,

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
