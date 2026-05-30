//! 字典聚合

use async_trait::async_trait;
use super::dept::CommonStatus;

#[derive(Debug, Clone)]
pub struct DictType { pub id: u64, pub name: String, pub dict_type: String, pub status: CommonStatus, pub remark: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }
impl DictType { pub fn new(name: String, dict_type: String) -> Self { Self { id: 0, name, dict_type, status: CommonStatus::Enable, remark: None, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } } }

#[derive(Debug, Clone)]
pub struct DictData { pub id: u64, pub sort: i32, pub label: String, pub value: String, pub dict_type: String, pub status: CommonStatus, pub color_type: Option<String>, pub css_class: Option<String>, pub remark: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }
impl DictData { pub fn new(dict_type: String, label: String, value: String, sort: i32) -> Self { Self { id: 0, sort, label, value, dict_type, status: CommonStatus::Enable, color_type: None, css_class: None, remark: None, creator: None, updater: None, created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(), deleted: 0 } } }

#[async_trait]
pub trait DictRepository: Send + Sync {
    async fn find_type_by_id(&self, id: u64) -> Result<Option<DictType>, anyhow::Error>;
    async fn find_type_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<DictType>, u64), anyhow::Error>;
    async fn save_type(&self, dt: &DictType) -> Result<(), anyhow::Error>;
    async fn delete_type(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_data_by_id(&self, id: u64) -> Result<Option<DictData>, anyhow::Error>;
    async fn find_data_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, anyhow::Error>;
    async fn save_data(&self, data: &DictData) -> Result<(), anyhow::Error>;
    async fn delete_data(&self, id: u64) -> Result<(), anyhow::Error>;
}
pub mod repo;
