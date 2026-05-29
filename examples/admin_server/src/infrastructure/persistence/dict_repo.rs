//! 字典仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::dict::{DictType, DictData};

/// 字典仓储 trait
#[async_trait]
pub trait DictRepository: Send + Sync {
    // DictType
    async fn find_type_by_id(&self, id: i64) -> Result<Option<DictType>, anyhow::Error>;
    async fn find_type_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<DictType>, u64), anyhow::Error>;
    async fn save_type(&self, dict_type: &DictType) -> Result<(), anyhow::Error>;
    async fn delete_type(&self, id: i64) -> Result<(), anyhow::Error>;
    // DictData
    async fn find_data_by_id(&self, id: i64) -> Result<Option<DictData>, anyhow::Error>;
    async fn find_data_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, anyhow::Error>;
    async fn save_data(&self, data: &DictData) -> Result<(), anyhow::Error>;
    async fn delete_data(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// 字典仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyDictRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl DictRepository for ToastyDictRepository {
    async fn find_type_by_id(&self, id: i64) -> Result<Option<DictType>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(DictType::find_by_id(db, id).await?)
    }

    async fn find_type_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<DictType>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = DictType::filter(DictType::deleted.eq(0i16));
        if let Some(kw) = keyword {
            stmt = stmt.filter(DictType::name.like(format!("%{}%", kw)));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(DictType::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn save_type(&self, dict_type: &DictType) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if dict_type.id == 0 {
            dict_type.clone().create(db).await?;
        } else {
            dict_type.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete_type(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut dt) = DictType::find_by_id(db, id).await? {
            dt.deleted = 1;
            dt.update(db).await?;
        }
        Ok(())
    }

    async fn find_data_by_id(&self, id: i64) -> Result<Option<DictData>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(DictData::find_by_id(db, id).await?)
    }

    async fn find_data_by_type(&self, dict_type: &str) -> Result<Vec<DictData>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(DictData::filter(DictData::dict_type.eq(dict_type).and(DictData::deleted.eq(0i16)))
            .order(DictData::sort.asc())
            .all(db)
            .await?)
    }

    async fn save_data(&self, data: &DictData) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if data.id == 0 {
            data.clone().create(db).await?;
        } else {
            data.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete_data(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut data) = DictData::find_by_id(db, id).await? {
            data.deleted = 1;
            data.update(db).await?;
        }
        Ok(())
    }
}
