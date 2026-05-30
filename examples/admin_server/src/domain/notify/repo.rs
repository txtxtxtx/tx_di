//! 站内信仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{NotifyTemplate, NotifyMessage, NotifyRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_notify_template"]
pub struct NotifyTemplateModel {
    #[key] #[auto] pub id: u64, pub name: String, #[unique] pub code: String,
    pub nickname: String, pub content: String, #[default(0u16)] pub template_type: u16,
    #[default("".to_string())] pub params: String, #[default(0u8)] pub status: u8, #[default("".to_string())] pub remark: String,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_notify_message"]
pub struct NotifyMessageModel {
    #[key] #[auto] pub id: u64, #[index] pub user_id: i64, #[default(0u8)] pub user_type: u8,
    pub template_id: i64, pub template_code: String, pub template_nickname: String,
    pub template_content: String, #[default(0u16)] pub template_type: u16,
    pub template_params: String, #[default(false)] pub read_status: bool,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<NotifyTemplateModel> for NotifyTemplate { fn from(m: NotifyTemplateModel) -> Self { Self { id: m.id, name: m.name, code: m.code, nickname: m.nickname, content: m.content, template_type: m.template_type, params: if m.params.is_empty() { None } else { Some(m.params) }, status: m.status, remark: if m.remark.is_empty() { None } else { Some(m.remark) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<NotifyMessageModel> for NotifyMessage { fn from(m: NotifyMessageModel) -> Self { Self { id: m.id, user_id: m.user_id as u64, user_type: m.user_type, template_id: m.template_id as u64, template_code: m.template_code, template_nickname: m.template_nickname, template_content: m.template_content, template_type: m.template_type, template_params: m.template_params, read_status: m.read_status, read_time: None, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyNotifyRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl NotifyRepository for ToastyNotifyRepository {
    async fn find_template_by_id(&self, id: u64) -> Result<Option<NotifyTemplate>, anyhow::Error> { let mut db = self.toasty.db().clone(); match NotifyTemplateModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(NotifyTemplate::from(m))), Err(_) => Ok(None) } }
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<NotifyTemplate>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = NotifyTemplateModel::all().count().exec(&mut db).await? as u64; let models = NotifyTemplateModel::all().offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(NotifyTemplate::from).collect(), total)) }
    async fn save_template(&self, tpl: &NotifyTemplate) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if tpl.id == 0 { toasty::create!(NotifyTemplateModel { name: tpl.name.clone(), code: tpl.code.clone(), nickname: tpl.nickname.clone(), content: tpl.content.clone(), template_type: tpl.template_type, params: tpl.params.clone().unwrap_or_default(), status: tpl.status, remark: tpl.remark.clone().unwrap_or_default(), creator: tpl.creator.clone().unwrap_or_default(), updater: tpl.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = NotifyTemplateModel::get_by_id(&mut db, tpl.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.name = tpl.name.clone(); m.code = tpl.code.clone(); m.nickname = tpl.nickname.clone(); m.content = tpl.content.clone(); m.template_type = tpl.template_type; m.params = tpl.params.clone().unwrap_or_default(); m.status = tpl.status; m.remark = tpl.remark.clone().unwrap_or_default(); m.creator = tpl.creator.clone().unwrap_or_default(); m.updater = tpl.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete_template(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match NotifyTemplateModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn save_message(&self, msg: &NotifyMessage) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); toasty::create!(NotifyMessageModel { user_id: msg.user_id as i64, user_type: msg.user_type, template_id: msg.template_id as i64, template_code: msg.template_code.clone(), template_nickname: msg.template_nickname.clone(), template_content: msg.template_content.clone(), template_type: msg.template_type, template_params: msg.template_params.clone(), read_status: msg.read_status, tenant_id: msg.tenant_id as i64, creator: msg.creator.clone().unwrap_or_default(), updater: msg.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(()) }
    async fn find_message_page(&self, user_id: u64, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<NotifyMessage>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = NotifyMessageModel::filter_by_user_id(user_id as i64).count().exec(&mut db).await? as u64; let models = NotifyMessageModel::filter_by_user_id(user_id as i64).offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0 && m.tenant_id == tenant_id as i64).map(NotifyMessage::from).collect(), total)) }
    async fn count_unread(&self, user_id: u64, tenant_id: u64) -> Result<u64, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = NotifyMessageModel::filter_by_user_id(user_id as i64).exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.tenant_id == tenant_id as i64 && !m.read_status && m.deleted == 0).count() as u64) }
    async fn mark_read(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match NotifyMessageModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.read_status = true; m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn mark_all_read(&self, user_id: u64, tenant_id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); let unread = NotifyMessageModel::filter_by_user_id(user_id as i64).exec(&mut db).await?; for mut m in unread { if m.tenant_id == tenant_id as i64 && !m.read_status && m.deleted == 0 { m.read_status = true; m.update().exec(&mut db).await?; } } Ok(()) }
}
