//! 配置仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::config::Config;

/// 配置仓储 trait
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Config>, anyhow::Error>;
    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, anyhow::Error>;
    async fn find_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Config>, u64), anyhow::Error>;
    async fn save(&self, config: &Config) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// 配置仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyConfigRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl ConfigRepository for ToastyConfigRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Config>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Config::find_by_id(db, id).await?)
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Config::filter(Config::config_key.eq(key).and(Config::deleted.eq(0i16))).first(db).await?)
    }

    async fn find_page(&self, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Config>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = Config::filter(Config::deleted.eq(0i16));
        if let Some(kw) = keyword {
            stmt = stmt.filter(Config::name.like(format!("%{}%", kw)));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let items = stmt.order(Config::id.desc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((items, total))
    }

    async fn save(&self, config: &Config) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if config.id == 0 {
            config.clone().create(db).await?;
        } else {
            config.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut c) = Config::find_by_id(db, id).await? {
            c.deleted = 1;
            c.update(db).await?;
        }
        Ok(())
    }
}
