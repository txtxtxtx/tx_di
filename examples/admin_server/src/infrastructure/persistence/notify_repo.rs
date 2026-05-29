//! 站内信仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::notify::{NotifyMessage, NotifyTemplate};

/// 站内信仓储 trait
#[async_trait]
pub trait NotifyRepository: Send + Sync {
    // NotifyTemplate
    async fn find_template_by_id(&self, id: i64) -> Result<Option<NotifyTemplate>, anyhow::Error>;
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<NotifyTemplate>, u64), anyhow::Error>;
    async fn save_template(&self, tpl: &NotifyTemplate) -> Result<(), anyhow::Error>;
    async fn delete_template(&self, id: i64) -> Result<(), anyhow::Error>;
    // NotifyMessage
    async fn save_message(&self, msg: &NotifyMessage) -> Result<(), anyhow::Error>;
    async fn find_message_page(&self, user_id: i64, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<NotifyMessage>, u64), anyhow::Error>;
    async fn count_unread(&self, user_id: i64, tenant_id: i64) -> Result<u64, anyhow::Error>;
    async fn mark_read(&self, id: i64) -> Result<(), anyhow::Error>;
    async fn mark_all_read(&self, user_id: i64, tenant_id: i64) -> Result<(), anyhow::Error>;
}

/// 站内信仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyNotifyRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl NotifyRepository for ToastyNotifyRepository {
    async fn find_template_by_id(&self, id: i64) -> Result<Option<NotifyTemplate>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(NotifyTemplate::find_by_id(db, id).await?)
    }

    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<NotifyTemplate>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = NotifyTemplate::filter(NotifyTemplate::deleted.eq(0i16));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(NotifyTemplate::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn save_template(&self, tpl: &NotifyTemplate) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if tpl.id == 0 {
            tpl.clone().create(db).await?;
        } else {
            tpl.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete_template(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut t) = NotifyTemplate::find_by_id(db, id).await? {
            t.deleted = 1;
            t.update(db).await?;
        }
        Ok(())
    }

    async fn save_message(&self, msg: &NotifyMessage) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        msg.clone().create(db).await?;
        Ok(())
    }

    async fn find_message_page(&self, user_id: i64, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<NotifyMessage>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = NotifyMessage::filter(
            NotifyMessage::user_id.eq(user_id)
                .and(NotifyMessage::tenant_id.eq(tenant_id))
                .and(NotifyMessage::deleted.eq(0i16))
        );
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(NotifyMessage::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn count_unread(&self, user_id: i64, tenant_id: i64) -> Result<u64, anyhow::Error> {
        let db = self.toasty.db();
        let count = NotifyMessage::filter(
            NotifyMessage::user_id.eq(user_id)
                .and(NotifyMessage::tenant_id.eq(tenant_id))
                .and(NotifyMessage::read_status.eq(false))
                .and(NotifyMessage::deleted.eq(0i16))
        ).count(db).await? as u64;
        Ok(count)
    }

    async fn mark_read(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut msg) = NotifyMessage::find_by_id(db, id).await? {
            msg.read_status = true;
            msg.read_time = Some(jiff::Timestamp::now());
            msg.update(db).await?;
        }
        Ok(())
    }

    async fn mark_all_read(&self, user_id: i64, tenant_id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        let unread = NotifyMessage::filter(
            NotifyMessage::user_id.eq(user_id)
                .and(NotifyMessage::tenant_id.eq(tenant_id))
                .and(NotifyMessage::read_status.eq(false))
                .and(NotifyMessage::deleted.eq(0i16))
        ).all(db).await?;
        for mut msg in unread {
            msg.read_status = true;
            msg.read_time = Some(jiff::Timestamp::now());
            msg.update(db).await?;
        }
        Ok(())
    }
}
