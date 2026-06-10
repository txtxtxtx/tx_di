use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDictTypeCommand {
    pub name: String,
    pub dict_type: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDictTypeCommand {
    pub id: u64,
    pub name: String,
    pub dict_type: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictTypeQueryRequest {
    pub name: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictTypeResponse {
    pub id: u64,
    pub name: String,
    pub dict_type: String,
    pub status: i32,
    pub remark: Option<String>,
}

impl From<admin_domain::dictionary::model::aggregate::DictType> for DictTypeResponse {
    fn from(dt: admin_domain::dictionary::model::aggregate::DictType) -> Self {
        Self {
            id: dt.id,
            name: dt.name,
            dict_type: dt.dict_type,
            status: dt.status,
            remark: dt.remark,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDictDataCommand {
    pub sort: i32,
    pub label: String,
    pub value: String,
    pub dict_type: String,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDictDataCommand {
    pub id: u64,
    pub sort: i32,
    pub label: String,
    pub value: String,
    pub dict_type: String,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictDataQueryRequest {
    pub dict_type: Option<String>,
    pub label: Option<String>,
    pub status: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictDataResponse {
    pub id: u64,
    pub sort: i32,
    pub label: String,
    pub value: String,
    pub dict_type: String,
    pub status: i32,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
}

impl From<admin_domain::dictionary::model::aggregate::DictData> for DictDataResponse {
    fn from(dd: admin_domain::dictionary::model::aggregate::DictData) -> Self {
        Self {
            id: dd.id,
            sort: dd.sort,
            label: dd.label,
            value: dd.value,
            dict_type: dd.dict_type,
            status: dd.status,
            color_type: dd.color_type,
            css_class: dd.css_class,
            remark: dd.remark,
        }
    }
}
