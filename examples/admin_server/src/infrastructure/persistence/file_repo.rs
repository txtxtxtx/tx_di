//! 文件仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::file::File;

/// 文件仓储 trait
#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<File>, anyhow::Error>;
    async fn find_page(&self, page: u64, page_size: u64) -> Result<(Vec<File>, u64), anyhow::Error>;
    async fn save(&self, file: &File) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// 文件仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyFileRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl FileRepository for ToastyFileRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<File>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(File::find_by_id(db, id).await?)
    }

    async fn find_page(&self, page: u64, page_size: u64) -> Result<(Vec<File>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let stmt = File::filter(File::deleted.eq(0i16));
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(File::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn save(&self, file: &File) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if file.id == 0 {
            file.clone().create(db).await?;
        } else {
            file.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut f) = File::find_by_id(db, id).await? {
            f.deleted = 1;
            f.update(db).await?;
        }
        Ok(())
    }
}
