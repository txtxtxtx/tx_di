//! 邮件仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{MailAccount, MailTemplate, MailLog, MailSendStatus, MailRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_mail_account"]
pub struct MailAccountModel {
    #[key] #[auto] pub id: u64, pub mail: String, pub username: String, pub password: String,
    pub host: String, #[default(25i32)] pub port: i32, #[default(false)] pub ssl_enable: bool, #[default(false)] pub starttls_enable: bool,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_mail_template"]
pub struct MailTemplateModel {
    #[key] #[auto] pub id: u64, pub name: String, #[unique] pub code: String, pub account_id: i64,
    #[default("".to_string())] pub nickname: String, #[default("".to_string())] pub title: String, #[default("".to_string())] pub content: String,
    #[default("".to_string())] pub params: String, #[default(0u8)] pub status: u8, #[default("".to_string())] pub remark: String,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_mail_log"]
pub struct MailLogModel {
    #[key] #[auto] pub id: u64, #[default(0i64)] #[index] pub user_id: i64, #[default(0u8)] pub user_type: u8,
    #[default("".to_string())] pub to_mails: String, #[default("".to_string())] pub cc_mails: String, #[default("".to_string())] pub bcc_mails: String,
    #[default(0i64)] pub account_id: i64, #[default("".to_string())] pub from_mail: String, #[default(0i64)] pub template_id: i64,
    #[default("".to_string())] pub template_code: String, #[default("".to_string())] pub template_nickname: String,
    #[default("".to_string())] pub template_title: String, #[default("".to_string())] pub template_content: String, #[default("".to_string())] pub template_params: String,
    pub send_status: MailSendStatus, #[default("".to_string())] pub send_message_id: String, #[default("".to_string())] pub send_exception: String,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<MailAccountModel> for MailAccount { fn from(m: MailAccountModel) -> Self { Self { id: m.id, mail: m.mail, username: m.username, password: m.password, host: m.host, port: m.port, ssl_enable: m.ssl_enable, starttls_enable: m.starttls_enable, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<MailTemplateModel> for MailTemplate { fn from(m: MailTemplateModel) -> Self { Self { id: m.id, name: m.name, code: m.code, account_id: m.account_id as u64, nickname: if m.nickname.is_empty() { None } else { Some(m.nickname) }, title: if m.title.is_empty() { None } else { Some(m.title) }, content: if m.content.is_empty() { None } else { Some(m.content) }, params: if m.params.is_empty() { None } else { Some(m.params) }, status: m.status, remark: if m.remark.is_empty() { None } else { Some(m.remark) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<MailLogModel> for MailLog { fn from(m: MailLogModel) -> Self { Self { id: m.id, user_id: if m.user_id == 0 { None } else { Some(m.user_id as u64) }, user_type: m.user_type, to_mails: if m.to_mails.is_empty() { None } else { Some(m.to_mails) }, cc_mails: if m.cc_mails.is_empty() { None } else { Some(m.cc_mails) }, bcc_mails: if m.bcc_mails.is_empty() { None } else { Some(m.bcc_mails) }, account_id: if m.account_id == 0 { None } else { Some(m.account_id as u64) }, from_mail: if m.from_mail.is_empty() { None } else { Some(m.from_mail) }, template_id: if m.template_id == 0 { None } else { Some(m.template_id as u64) }, template_code: if m.template_code.is_empty() { None } else { Some(m.template_code) }, template_nickname: if m.template_nickname.is_empty() { None } else { Some(m.template_nickname) }, template_title: if m.template_title.is_empty() { None } else { Some(m.template_title) }, template_content: if m.template_content.is_empty() { None } else { Some(m.template_content) }, template_params: if m.template_params.is_empty() { None } else { Some(m.template_params) }, send_status: m.send_status, send_time: None, send_message_id: if m.send_message_id.is_empty() { None } else { Some(m.send_message_id) }, send_exception: if m.send_exception.is_empty() { None } else { Some(m.send_exception) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyMailRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl MailRepository for ToastyMailRepository {
    async fn find_account_by_id(&self, id: u64) -> Result<Option<MailAccount>, anyhow::Error> { let mut db = self.toasty.db().clone(); match MailAccountModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(MailAccount::from(m))), Err(_) => Ok(None) } }
    async fn find_all_accounts(&self) -> Result<Vec<MailAccount>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(MailAccountModel::all().exec(&mut db).await?.into_iter().filter(|m| m.deleted == 0).map(MailAccount::from).collect()) }
    async fn save_account(&self, account: &MailAccount) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if account.id == 0 { toasty::create!(MailAccountModel { mail: account.mail.clone(), username: account.username.clone(), password: account.password.clone(), host: account.host.clone(), port: account.port, ssl_enable: account.ssl_enable, starttls_enable: account.starttls_enable, creator: account.creator.clone().unwrap_or_default(), updater: account.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = MailAccountModel::get_by_id(&mut db, account.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.mail = account.mail.clone(); m.username = account.username.clone(); m.password = account.password.clone(); m.host = account.host.clone(); m.port = account.port; m.ssl_enable = account.ssl_enable; m.starttls_enable = account.starttls_enable; m.creator = account.creator.clone().unwrap_or_default(); m.updater = account.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete_account(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match MailAccountModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn find_template_by_id(&self, id: u64) -> Result<Option<MailTemplate>, anyhow::Error> { let mut db = self.toasty.db().clone(); match MailTemplateModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(MailTemplate::from(m))), Err(_) => Ok(None) } }
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailTemplate>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = MailTemplateModel::all().count().exec(&mut db).await? as u64; let models = MailTemplateModel::all().offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(MailTemplate::from).collect(), total)) }
    async fn save_template(&self, tpl: &MailTemplate) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if tpl.id == 0 { toasty::create!(MailTemplateModel { name: tpl.name.clone(), code: tpl.code.clone(), account_id: tpl.account_id as i64, nickname: tpl.nickname.clone().unwrap_or_default(), title: tpl.title.clone().unwrap_or_default(), content: tpl.content.clone().unwrap_or_default(), params: tpl.params.clone().unwrap_or_default(), status: tpl.status, remark: tpl.remark.clone().unwrap_or_default(), creator: tpl.creator.clone().unwrap_or_default(), updater: tpl.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = MailTemplateModel::get_by_id(&mut db, tpl.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.name = tpl.name.clone(); m.code = tpl.code.clone(); m.account_id = tpl.account_id as i64; m.nickname = tpl.nickname.clone().unwrap_or_default(); m.title = tpl.title.clone().unwrap_or_default(); m.content = tpl.content.clone().unwrap_or_default(); m.params = tpl.params.clone().unwrap_or_default(); m.status = tpl.status; m.remark = tpl.remark.clone().unwrap_or_default(); m.creator = tpl.creator.clone().unwrap_or_default(); m.updater = tpl.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete_template(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match MailTemplateModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn save_log(&self, log: &MailLog) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); toasty::create!(MailLogModel { user_id: log.user_id.map(|v| v as i64).unwrap_or_default(), user_type: log.user_type, to_mails: log.to_mails.clone().unwrap_or_default(), cc_mails: log.cc_mails.clone().unwrap_or_default(), bcc_mails: log.bcc_mails.clone().unwrap_or_default(), account_id: log.account_id.map(|v| v as i64).unwrap_or_default(), from_mail: log.from_mail.clone().unwrap_or_default(), template_id: log.template_id.map(|v| v as i64).unwrap_or_default(), template_code: log.template_code.clone().unwrap_or_default(), template_nickname: log.template_nickname.clone().unwrap_or_default(), template_title: log.template_title.clone().unwrap_or_default(), template_content: log.template_content.clone().unwrap_or_default(), template_params: log.template_params.clone().unwrap_or_default(), send_status: log.send_status, send_message_id: log.send_message_id.clone().unwrap_or_default(), send_exception: log.send_exception.clone().unwrap_or_default(), creator: log.creator.clone().unwrap_or_default(), updater: log.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(()) }
    async fn find_log_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailLog>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = MailLogModel::all().count().exec(&mut db).await? as u64; let models = MailLogModel::all().offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(MailLog::from).collect(), total)) }
}
