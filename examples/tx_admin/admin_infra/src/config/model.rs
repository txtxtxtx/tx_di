use toasty::Model;

use crate::common::Deleted;

/// 系统配置表
#[derive(Debug, Clone, Model)]
#[table = "sys_config"]
pub struct SysConfig {
    #[key]
    #[auto]
    pub id: u64,

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

    #[auto]
    pub created_at: jiff::Timestamp,

    #[default("".to_string())]
    pub updater: String,

    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}
