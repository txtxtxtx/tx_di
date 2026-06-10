//! 字典 DTO

use serde::{Deserialize, Serialize};

// ── 字典类型 ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DictTypeDto {
    pub id: u64,
    pub name: String,
    pub dict_type: String,
    pub status: String,
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub created_at: String,
    pub updater: Option<String>,
    pub updated_at: String,
}

impl From<&crate::domain::dict::DictType> for DictTypeDto {
    fn from(d: &crate::domain::dict::DictType) -> Self {
        Self {
            id: d.id,
            name: d.name.clone(),
            dict_type: d.dict_type.clone(),
            status: d.status.to_string(),
            remark: d.remark.clone(),
            creator: d.creator.clone(),
            created_at: d.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updater: d.updater.clone(),
            updated_at: d.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDictTypeRequest {
    pub name: String,
    pub dict_type: String,
    pub status: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDictTypeRequest {
    pub name: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

// ── 字典数据 ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DictDataDto {
    pub id: u64,
    pub sort: i32,
    pub label: String,
    pub value: String,
    pub dict_type: String,
    pub status: String,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub created_at: String,
    pub updater: Option<String>,
    pub updated_at: String,
}

impl From<&crate::domain::dict::DictData> for DictDataDto {
    fn from(d: &crate::domain::dict::DictData) -> Self {
        Self {
            id: d.id,
            sort: d.sort,
            label: d.label.clone(),
            value: d.value.clone(),
            dict_type: d.dict_type.clone(),
            status: d.status.to_string(),
            color_type: d.color_type.clone(),
            css_class: d.css_class.clone(),
            remark: d.remark.clone(),
            creator: d.creator.clone(),
            created_at: d.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updater: d.updater.clone(),
            updated_at: d.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDictDataRequest {
    pub sort: Option<i32>,
    pub label: String,
    pub value: String,
    pub dict_type: String,
    pub status: Option<String>,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDictDataRequest {
    pub sort: Option<i32>,
    pub label: Option<String>,
    pub value: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<String>,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
}
