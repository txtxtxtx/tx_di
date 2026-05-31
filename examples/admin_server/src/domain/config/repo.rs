//! 配置仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{Config, ConfigType, ConfigRepository};

#[derive(Debug, Clone, Model)]
#[table = "infra_config"]
pub struct ConfigModel {
    #[key] #[auto] pub id: u64, #[default("".to_string())] pub category: String, pub config_type: ConfigType,
    pub name: String, #[unique] pub config_key: String, #[default("".to_string())] pub value: String,
    #[default(true)] pub visible: bool, #[default("".to_string())] pub remark: String,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<ConfigModel> for Config { fn from(m: ConfigModel) -> Self { Self { id: m.id, category: if m.category.is_empty() { None } else { Some(m.category) }, config_type: m.config_type, name: m.name, config_key: m.config_key, value: if m.value.is_empty() { None } else { Some(m.value) }, visible: m.visible, remark: if m.remark.is_empty() { None } else { Some(m.remark) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyConfigRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl ConfigRepository for ToastyConfigRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Config>, anyhow::Error> { let mut db = self.toasty.db().clone(); match ConfigModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Config::from(m))), Err(_) => Ok(None) } }
    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(ConfigModel::filter_by_config_key(key.to_string()).first().exec(&mut db).await?.map(Config::from)) }
    async fn find_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Config>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = ConfigModel::all().count().exec(&mut db).await? as u64; let models = ConfigModel::all().offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(Config::from).collect(), total)) }
    async fn save(&self, config: &Config) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if config.id == 0 { toasty::create!(ConfigModel { category: config.category.clone().unwrap_or_default(), config_type: config.config_type, name: config.name.clone(), config_key: config.config_key.clone(), value: config.value.clone().unwrap_or_default(), visible: config.visible, remark: config.remark.clone().unwrap_or_default(), creator: config.creator.clone().unwrap_or_default(), updater: config.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = ConfigModel::get_by_id(&mut db, config.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.category = config.category.clone().unwrap_or_default(); m.config_type = config.config_type; m.name = config.name.clone(); m.config_key = config.config_key.clone(); m.value = config.value.clone().unwrap_or_default(); m.visible = config.visible; m.remark = config.remark.clone().unwrap_or_default(); m.creator = config.creator.clone().unwrap_or_default(); m.updater = config.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match ConfigModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
