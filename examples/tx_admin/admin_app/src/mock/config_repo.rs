use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::config::model::aggregate::Config;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::repository::ConfigRepository;
use admin_domain::shared::repository::RepositoryError;
use admin_domain::shared::model::value_object::DeletedStatus;
use tx_common::page::Page;
use tx_di_core::{tx_comp, tx_cst};
use tx_error::AppResult;

#[tx_comp(as_trait = dyn ConfigRepository)]
pub struct MockConfigRepository {
    #[tx_cst(RwLock::new(HashMap::new()))]
    configs: RwLock<HashMap<u64, Config>>,
}

impl MockConfigRepository {
    pub fn new() -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MockConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigRepository for MockConfigRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Config>> {
        let configs = self.configs.read().unwrap();
        Ok(configs.get(&id).filter(|c| c.audit.deleted == DeletedStatus::Normal).cloned())
    }

    async fn find_by_key(&self, key: &str) -> AppResult<Option<Config>> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .find(|c| c.config_key == key && c.audit.deleted == DeletedStatus::Normal)
            .cloned())
    }

    async fn find_page(
        &self,
        query: &ConfigQuery,
        page: Page<Config>,
    ) -> AppResult<Page<Config>> {
        let configs = self.configs.read().unwrap();
        let filtered: Vec<Config> = configs
            .values()
            .filter(|c| c.audit.deleted == DeletedStatus::Normal)
            .filter(|c| {
                if let Some(ref name) = query.name {
                    if !c.name.contains(name.as_str()) {
                        return false;
                    }
                }
                if let Some(ref category) = query.category {
                    if c.category != *category {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = filtered.len() as i64;
        let offset = page.offset() as usize;
        let list = filtered
            .into_iter()
            .skip(offset)
            .take(page.size as usize)
            .collect();

        Ok(Page::new(list, page.page, page.size, total))
    }

    async fn find_all(&self, query: &ConfigQuery) -> AppResult<Vec<Config>> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .filter(|c| c.audit.deleted == DeletedStatus::Normal)
            .filter(|c| {
                if let Some(ref category) = query.category {
                    if c.category != *category {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect())
    }

    async fn insert(&self, config: &Config) -> AppResult<()> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn update(&self, config: &Config) -> AppResult<()> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut configs = self.configs.write().unwrap();
        if let Some(config) = configs.get_mut(&id) {
            config.audit.deleted = DeletedStatus::Deleted;
            Ok(())
        } else {
            Err(RepositoryError::NotFound)?
        }
    }

    async fn exists_by_key(&self, key: &str) -> AppResult<bool> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .any(|c| c.config_key == key && c.audit.deleted == DeletedStatus::Normal))
    }

    async fn find_by_keys(&self, keys: &[String]) -> AppResult<Vec<Config>> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .filter(|c| c.audit.deleted == DeletedStatus::Normal)
            .filter(|c| keys.contains(&c.config_key))
            .cloned()
            .collect())
    }
}
