use toasty::Model;

use crate::common::{Status, Deleted};

/// 字典类型表
#[derive(Debug, Clone, Model)]
#[table = "sys_dict_type"]
pub struct SysDictType {
    #[key]
    #[auto]
    pub id: u64,

    #[default("".to_string())]
    pub name: String,

    #[unique]
    pub dict_type: String,

    #[default(Status::Disabled)]
    pub status: Status,

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

/// 字典数据表
#[derive(Debug, Clone, Model)]
#[table = "sys_dict_data"]
pub struct SysDictData {
    #[key]
    #[auto]
    pub id: u64,

    #[default(0)]
    pub sort: i32,

    #[default("".to_string())]
    pub label: String,

    #[default("".to_string())]
    pub value: String,

    #[index]
    pub dict_type: String,

    #[default(Status::Disabled)]
    pub status: Status,

    #[default("".to_string())]
    pub color_type: String,

    #[default("".to_string())]
    pub css_class: String,

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
