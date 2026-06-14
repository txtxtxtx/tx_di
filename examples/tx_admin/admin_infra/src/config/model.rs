use toasty::Model;

/// 系统配置表
#[derive(Debug, Clone, Model)]
#[table = "sys_config"]
pub struct SysConfig {
    #[key]
    #[auto]
    pub id: i64,

    #[default("".to_string())]
    pub category: String,

    #[default(0)]
    pub config_type: i32,

    #[default("".to_string())]
    pub name: String,

    #[unique]
    pub config_key: String,

    #[default("".to_string())]
    pub value: String,

    #[default(1)]
    pub visible: i32,

    #[default("".to_string())]
    pub remark: String,

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
