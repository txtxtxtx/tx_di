use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::config::model::aggregate::Config;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::repository::ConfigRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::SysConfig;
use crate::common::Deleted;

/// Toasty 实现的 ConfigRepository
#[tx_comp(as_trait = dyn ConfigRepository)]
pub struct ToastyConfigRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyConfigRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(c: &SysConfig) -> Config {
        Config::restore(
            c.id as u64,
            c.category.clone(),
            c.config_type,
            c.name.clone(),
            c.config_key.clone(),
            c.value.clone(),
            c.visible,
            if c.remark.is_empty() { None } else { Some(c.remark.clone()) },
            AuditFields {
                creator: if c.creator.is_empty() { None } else { Some(c.creator.clone()) },
                create_time: c.created_at.parse().unwrap_or_default(),
                updater: if c.updater.is_empty() { None } else { Some(c.updater.clone()) },
                update_time: c.updated_at.parse().unwrap_or_default(),
                deleted: if c.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl ConfigRepository for ToastyConfigRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Config>> {
        let mut db = self.plugin.db().clone();
        match SysConfig::get_by_id(&mut db, id as i64).await {
            Ok(c) if c.deleted == Deleted::No => Ok(Some(Self::to_domain(&c))),
            _ => Ok(None),
        }
    }

    async fn find_by_key(&self, key: &str) -> AppResult<Option<Config>> {
        let mut db = self.plugin.db().clone();
        let config = SysConfig::filter_by_config_key(key)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        match config {
            Some(c) if c.deleted == Deleted::No => Ok(Some(Self::to_domain(&c))),
            _ => Ok(None),
        }
    }

    async fn find_by_keys(&self, keys: &[String]) -> AppResult<Vec<Config>> {
        let mut db = self.plugin.db().clone();
        let all = SysConfig::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|c| c.deleted == Deleted::No && keys.contains(&c.config_key))
            .map(Self::to_domain)
            .collect())
    }

    async fn find_page(&self, query: &ConfigQuery, page: Page<Config>) -> AppResult<Page<Config>> {
        let mut db = self.plugin.db().clone();
        let all = SysConfig::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        let filtered: Vec<&SysConfig> = all
            .iter()
            .filter(|c| c.deleted == Deleted::No)
            .filter(|c| {
                if let Some(ref name) = query.name {
                    if !c.name.contains(name.as_str()) { return false; }
                }
                if let Some(ref category) = query.category {
                    if c.category != *category { return false; }
                }
                if let Some(ref config_key) = query.config_key {
                    if !c.config_key.contains(config_key.as_str()) { return false; }
                }
                if let Some(config_type) = query.config_type {
                    if c.config_type != config_type { return false; }
                }
                true
            })
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let size = page.size as usize;

        let list: Vec<Config> = filtered
            .into_iter()
            .skip(offset)
            .take(size)
            .map(Self::to_domain)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn find_all(&self, query: &ConfigQuery) -> AppResult<Vec<Config>> {
        let mut db = self.plugin.db().clone();
        let all = SysConfig::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|c| c.deleted == Deleted::No)
            .filter(|c| {
                if let Some(ref category) = query.category {
                    if c.category != *category { return false; }
                }
                if let Some(ref name) = query.name {
                    if !c.name.contains(name.as_str()) { return false; }
                }
                true
            })
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, config: &Config) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysConfig::create()
            .id(config.id as i64)
            .category(config.category.clone())
            .config_type(config.config_type)
            .name(config.name.clone())
            .config_key(config.config_key.clone())
            .value(config.value.clone())
            .visible(config.visible)
            .remark(config.remark.clone().unwrap_or_default())
            .creator(config.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(config.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn update(&self, config: &Config) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysConfig::get_by_id(&mut db, config.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .category(config.category.clone())
            .config_type(config.config_type)
            .name(config.name.clone())
            .config_key(config.config_key.clone())
            .value(config.value.clone())
            .visible(config.visible)
            .remark(config.remark.clone().unwrap_or_default())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(config.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut config = SysConfig::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        config.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn exists_by_key(&self, key: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let config = SysConfig::filter_by_config_key(key)
            .first()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(config.map(|c| c.deleted == Deleted::No).unwrap_or(false))
    }
}
