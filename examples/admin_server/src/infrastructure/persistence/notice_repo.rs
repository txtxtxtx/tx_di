//! 通知公告仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::notice::Notice;

/// 通知公告仓储 trait
#[async_trait]
pub trait NoticeRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Notice>, anyhow::Error>;
    async fn find_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<Notice>, u64), anyhow::Error>;
    async fn save(&self, notice: &Notice) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// 通知公告仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyNoticeRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl NoticeRepository for ToastyNoticeRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Notice>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Notice::find_by_id(db, id).await?)
    }

    async fn find_page(&self, tenant_id: i64, page: u64, page_size: u64) -> Result<(Vec<Notice>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = Notice::filter(Notice::tenant_id.eq(tenant_id).and(Notice::deleted.eq(0i16)));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(Notice::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn save(&self, notice: &Notice) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if notice.id == 0 {
            notice.clone().create(db).await?;
        } else {
            notice.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut n) = Notice::find_by_id(db, id).await? {
            n.deleted = 1;
            n.update(db).await?;
        }
        Ok(())
    }
}
