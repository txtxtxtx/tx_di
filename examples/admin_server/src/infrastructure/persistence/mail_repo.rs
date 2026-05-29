//! 邮件仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::mail::{MailAccount, MailTemplate, MailLog};

/// 邮件仓储 trait
#[async_trait]
pub trait MailRepository: Send + Sync {
    // MailAccount
    async fn find_account_by_id(&self, id: i64) -> Result<Option<MailAccount>, anyhow::Error>;
    async fn find_all_accounts(&self) -> Result<Vec<MailAccount>, anyhow::Error>;
    async fn save_account(&self, account: &MailAccount) -> Result<(), anyhow::Error>;
    async fn delete_account(&self, id: i64) -> Result<(), anyhow::Error>;
    // MailTemplate
    async fn find_template_by_id(&self, id: i64) -> Result<Option<MailTemplate>, anyhow::Error>;
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailTemplate>, u64), anyhow::Error>;
    async fn save_template(&self, tpl: &MailTemplate) -> Result<(), anyhow::Error>;
    async fn delete_template(&self, id: i64) -> Result<(), anyhow::Error>;
    // MailLog
    async fn save_log(&self, log: &MailLog) -> Result<(), anyhow::Error>;
    async fn find_log_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailLog>, u64), anyhow::Error>;
}

/// 邮件仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyMailRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl MailRepository for ToastyMailRepository {
    async fn find_account_by_id(&self, id: i64) -> Result<Option<MailAccount>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(MailAccount::find_by_id(db, id).await?)
    }

    async fn find_all_accounts(&self) -> Result<Vec<MailAccount>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(MailAccount::filter(MailAccount::deleted.eq(0i16)).all(db).await?)
    }

    async fn save_account(&self, account: &MailAccount) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if account.id == 0 {
            account.clone().create(db).await?;
        } else {
            account.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete_account(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut a) = MailAccount::find_by_id(db, id).await? {
            a.deleted = 1;
            a.update(db).await?;
        }
        Ok(())
    }

    async fn find_template_by_id(&self, id: i64) -> Result<Option<MailTemplate>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(MailTemplate::find_by_id(db, id).await?)
    }

    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailTemplate>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = MailTemplate::filter(MailTemplate::deleted.eq(0i16));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(MailTemplate::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn save_template(&self, tpl: &MailTemplate) -> Result<(), anyhow::Error> {
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
        if let Some(mut t) = MailTemplate::find_by_id(db, id).await? {
            t.deleted = 1;
            t.update(db).await?;
        }
        Ok(())
    }

    async fn save_log(&self, log: &MailLog) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        log.clone().create(db).await?;
        Ok(())
    }

    async fn find_log_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailLog>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = MailLog::filter(MailLog::deleted.eq(0i16));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let logs = stmt.order(MailLog::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((logs, total))
    }
}
