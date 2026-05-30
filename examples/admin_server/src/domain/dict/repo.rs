//! 字典仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{DictType, DictData, DictRepository};
use super::super::dept::CommonStatus;

#[derive(Debug, Clone, Model)]
#[table = "system_dict_type"]
pub struct DictTypeModel {
    #[key] #[auto] pub id: u64, pub name: String, #[unique] pub dict_type: String,
    pub status: String, #[default("".to_string())] pub remark: String, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_dict_data"]
pub struct DictDataModel {
    #[key] #[auto] pub id: u64, #[default(0i32)] pub sort: i32, pub label: String, pub value: String,
    #[index] pub dict_type: String, pub status: String,
    #[default("".to_string())] pub color_type: String, #[default("".to_string())] pub css_class: String, #[default("".to_string())] pub remark: String,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

fn cs(s: &str) -> CommonStatus { if s == "disable" { CommonStatus::Disable } else { CommonStatus::Enable } }

impl From<DictTypeModel> for DictType { fn from(m: DictTypeModel) -> Self { Self { id: m.id, name: m.name, dict_type: m.dict_type, status: cs(&m.status), remark: if m.remark.is_empty() { None } else { Some(m.remark) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<DictDataModel> for DictData { fn from(m: DictDataModel) -> Self { Self { id: m.id, sort: m.sort, label: m.label, value: m.value, dict_type: m.dict_type, status: cs(&m.status), color_type: if m.color_type.is_empty() { None } else { Some(m.color_type) }, css_class: if m.css_class.is_empty() { None } else { Some(m.css_class) }, remark: if m.remark.is_empty() { None } else { Some(m.remark) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyDictRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl DictRepository for ToastyDictRepository {
    async fn find_type_by_id(&self, id: u64) -> Result<Option<DictType>, anyhow::Error> { let mut db = self.toasty.db().clone(); match DictTypeModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(DictType::from(m))), Err(_) => Ok(None) } }
    async fn find_type_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<DictType>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = DictTypeModel::all().count().exec(&mut db).await? as u64; let models = DictTypeModel::all().offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(DictType::from).collect(), total)) }
    async fn save_type(&self, dt: &DictType) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if dt.id == 0 { toasty::create!(DictTypeModel { name: dt.name.clone(), dict_type: dt.dict_type.clone(), status: dt.status.to_string(), remark: dt.remark.clone().unwrap_or_default(), creator: dt.creator.clone().unwrap_or_default(), updater: dt.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = DictTypeModel::get_by_id(&mut db, dt.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.name = dt.name.clone(); m.dict_type = dt.dict_type.clone(); m.status = dt.status.to_string(); m.remark = dt.remark.clone().unwrap_or_default(); m.creator = dt.creator.clone().unwrap_or_default(); m.updater = dt.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete_type(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match DictTypeModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn find_data_by_id(&self, id: u64) -> Result<Option<DictData>, anyhow::Error> { let mut db = self.toasty.db().clone(); match DictDataModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(DictData::from(m))), Err(_) => Ok(None) } }
    async fn find_data_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = DictDataModel::filter_by_dict_type(dict_type.to_string()).exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.deleted == 0).map(DictData::from).collect()) }
    async fn save_data(&self, data: &DictData) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if data.id == 0 { toasty::create!(DictDataModel { sort: data.sort, label: data.label.clone(), value: data.value.clone(), dict_type: data.dict_type.clone(), status: data.status.to_string(), color_type: data.color_type.clone().unwrap_or_default(), css_class: data.css_class.clone().unwrap_or_default(), remark: data.remark.clone().unwrap_or_default(), creator: data.creator.clone().unwrap_or_default(), updater: data.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = DictDataModel::get_by_id(&mut db, data.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.sort = data.sort; m.label = data.label.clone(); m.value = data.value.clone(); m.dict_type = data.dict_type.clone(); m.status = data.status.to_string(); m.color_type = data.color_type.clone().unwrap_or_default(); m.css_class = data.css_class.clone().unwrap_or_default(); m.remark = data.remark.clone().unwrap_or_default(); m.creator = data.creator.clone().unwrap_or_default(); m.updater = data.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete_data(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match DictDataModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
