use toasty::Model;

use crate::common::{Deleted, StorageType};

/// 系统文件表
#[derive(Debug, Clone, Model)]
#[table = "sys_file"]
pub struct SysFile {
    #[key]
    #[auto]
    pub id: i64,

    #[default(0)]
    pub config_id: i32,

    #[default("".to_string())]
    pub name: String,

    #[default("".to_string())]
    pub file_path: String,

    #[default("".to_string())]
    pub url: String,

    #[default("".to_string())]
    pub file_type: String,

    #[default(0)]
    pub size: i32,

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

/// 文件存储配置表
#[derive(Debug, Clone, Model)]
#[table = "sys_file_config"]
pub struct SysFileConfig {
    #[key]
    #[auto]
    pub id: i32,

    #[default("".to_string())]
    pub name: String,

    #[default(StorageType::Local)]
    pub storage: StorageType,

    #[default("".to_string())]
    pub remark: String,

    #[default(0)]
    pub master: i32,

    #[default("".to_string())]
    pub config: String,

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
