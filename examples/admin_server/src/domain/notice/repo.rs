//! 通知公告仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{Notice, NoticeType, NoticeRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_notice"]
pub struct NoticeModel {
    #[key] #[auto] pub id: u64, pub title: String, pub content: String, pub notice_type: NoticeType,
    #[default(0u8)] pub status: u8, #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<NoticeModel> for Notice { fn from(m: NoticeModel) -> Self { Self { id: m.id, title: m.title, content: m.content, notice_type: m.notice_type, status: m.status, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyNoticeRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl NoticeRepository for ToastyNoticeRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Notice>, anyhow::Error> { let mut db = self.toasty.db().clone(); match NoticeModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Notice::from(m))), Err(_) => Ok(None) } }
    async fn find_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<Notice>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = NoticeModel::filter_by_tenant_id(tenant_id as i64).count().exec(&mut db).await? as u64; let models = NoticeModel::filter_by_tenant_id(tenant_id as i64).offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(Notice::from).collect(), total)) }
    async fn save(&self, notice: &Notice) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if notice.id == 0 { toasty::create!(NoticeModel { title: notice.title.clone(), content: notice.content.clone(), notice_type: notice.notice_type, status: notice.status, tenant_id: notice.tenant_id as i64, creator: notice.creator.clone().unwrap_or_default(), updater: notice.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = NoticeModel::get_by_id(&mut db, notice.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.title = notice.title.clone(); m.content = notice.content.clone(); m.notice_type = notice.notice_type; m.status = notice.status; m.tenant_id = notice.tenant_id as i64; m.creator = notice.creator.clone().unwrap_or_default(); m.updater = notice.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match NoticeModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
