//! 字典聚合
//!
//! 字典类型和字典数据，用于系统配置的下拉选项。

use async_trait::async_trait;
use toasty::Model;
use super::dept::CommonStatus;

/// 字典类型实体
#[derive(Debug, Clone, Model)]
#[table = "system_dict_type"]
pub struct DictType {
    #[key]
    #[auto]
    pub id: u64,
    pub name: String,
    #[unique]
    pub dict_type: String,
    pub status: CommonStatus,
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
    pub deleted_time: Option<jiff::Timestamp>,
}

impl DictType {
    pub fn new(name: String, dict_type: String) -> Self {
        Self {
            id: 0, name, dict_type, status: CommonStatus::Enable,
            remark: None, creator: None, updater: None,
            created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(),
            deleted: 0, deleted_time: None,
        }
    }
}

/// 字典数据实体
#[derive(Debug, Clone, Model)]
#[table = "system_dict_data"]
pub struct DictData {
    #[key]
    #[auto]
    pub id: u64,
    #[default(0i32)]
    pub sort: i32,
    pub label: String,
    pub value: String,
    #[index]
    pub dict_type: String,
    pub status: CommonStatus,
    pub color_type: Option<String>,
    pub css_class: Option<String>,
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

impl DictData {
    pub fn new(dict_type: String, label: String, value: String, sort: i32) -> Self {
        Self {
            id: 0, sort, label, value, dict_type, status: CommonStatus::Enable,
            color_type: None, css_class: None, remark: None,
            creator: None, updater: None,
            created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }
}

#[async_trait]
pub trait DictRepository: Send + Sync {
    async fn find_type_by_id(&self, id: u64) -> Result<Option<DictType>, anyhow::Error>;
    async fn find_type_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<DictType>, u64), anyhow::Error>;
    async fn save_type(&self, dict_type: &DictType) -> Result<(), anyhow::Error>;
    async fn delete_type(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_data_by_id(&self, id: u64) -> Result<Option<DictData>, anyhow::Error>;
    async fn find_data_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, anyhow::Error>;
    async fn save_data(&self, data: &DictData) -> Result<(), anyhow::Error>;
    async fn delete_data(&self, id: u64) -> Result<(), anyhow::Error>;
}
