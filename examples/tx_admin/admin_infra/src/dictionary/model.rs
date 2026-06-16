use toasty::Model;

use crate::common::{Status, Deleted};

/// 字典类型表
#[derive(Debug, Clone, Model)]
#[table = "sys_dict_type"]
pub struct SysDictType {
    #[key]
    #[auto]
    pub id: i64,

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

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}

/// 字典数据表
#[derive(Debug, Clone, Model)]
#[table = "sys_dict_data"]
pub struct SysDictData {
    #[key]
    #[auto]
    pub id: i64,

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

    #[default("".to_string())]
    pub created_at: String,

    #[default("".to_string())]
    pub updater: String,

    #[default("".to_string())]
    pub updated_at: String,

    #[default(Deleted::No)]
    pub deleted: Deleted,
}
