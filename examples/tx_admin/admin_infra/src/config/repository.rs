use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::config::model::aggregate::Config;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::repository::ConfigRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::{RepositoryError, db_err};
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::SysConfig;
use crate::common::Deleted;

/// Toasty 实现的 ConfigRepository
#[derive(Component)]
#[component(as_trait = dyn ConfigRepository)]
pub struct ToastyConfigRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyConfigRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(c: &SysConfig) -> Config {
        Config::restore(
            c.id,
            c.category.clone(),
            c.config_type,
            c.name.clone(),
            c.config_key.clone(),
            c.value.clone(),
            c.visible,
            if c.remark.is_empty() { None } else { Some(c.remark.clone()) },
            AuditFields {
                creator: if c.creator.is_empty() { None } else { Some(c.creator.clone()) },
                create_time: c.created_at,
                updater: if c.updater.is_empty() { None } else { Some(c.updater.clone()) },
                update_time: c.updated_at,
                deleted: if c.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl ConfigRepository for ToastyConfigRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Config>> {
        let mut db = self.plugin.db().clone();
        match SysConfig::get_by_id(&mut db, id).await {
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
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;
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
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;

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
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;

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
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;

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
        SysConfig::create()
            .id(config.id)
            .category(config.category.clone())
            .config_type(config.config_type)
            .name(config.name.clone())
            .config_key(config.config_key.clone())
            .value(config.value.clone())
            .visible(config.visible)
            .remark(config.remark.clone().unwrap_or_default())
            .creator(config.audit.creator.clone().unwrap_or_default())
            .updater(config.audit.updater.clone().unwrap_or_default())
            .deleted(Deleted::from(config.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;
        Ok(())
    }

    async fn update(&self, config: &Config) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysConfig::get_by_id(&mut db, config.id)
            .await
            .map_err(|_| RepositoryError::NotFoundConfig)?;

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
            .deleted(Deleted::from(config.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut config = SysConfig::get_by_id(&mut db, id)
            .await
            .map_err(|_| RepositoryError::NotFoundConfig)?;

        config.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;
        Ok(())
    }

    async fn exists_by_key(&self, key: &str) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let config = SysConfig::filter_by_config_key(key)
            .first()
            .exec(&mut db)
            .await
            .map_err(|e| db_err(e, RepositoryError::DatabaseConfig))?;
        Ok(config.map(|c| c.deleted == Deleted::No).unwrap_or(false))
    }
}
