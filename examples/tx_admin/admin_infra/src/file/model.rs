use toasty::Model;

use crate::common::{Deleted, StorageType};

/// 系统文件表
#[derive(Debug, Clone, Model)]
#[table = "sys_file"]
pub struct SysFile {
    #[key]
    #[auto]
    pub id: u64,

    #[default(0)]
    pub config_id: u64,

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

    #[auto]
    pub created_at: jiff::Timestamp,

    #[default("".to_string())]
    pub updater: String,

    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}

/// 文件存储配置表
#[derive(Debug, Clone, Model)]
#[table = "sys_file_config"]
pub struct SysFileConfig {
    #[key]
    #[auto]
    pub id: u64,

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

    #[auto]
    pub created_at: jiff::Timestamp,

    #[default("".to_string())]
    pub updater: String,

    #[update(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}
