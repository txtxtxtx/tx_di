use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use admin_domain::config::model::aggregate::Config;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::repository::ConfigRepository;
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

pub struct MockConfigRepository {
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
    async fn find_by_id(&self, id: u64) -> Result<Option<Config>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs.get(&id).filter(|c| c.audit.deleted == 0).cloned())
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<Config>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .find(|c| c.config_key == key && c.audit.deleted == 0)
            .cloned())
    }

    async fn find_page(
        &self,
        query: &ConfigQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<Config>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        let filtered: Vec<Config> = configs
            .values()
            .filter(|c| c.audit.deleted == 0)
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
            .take(page.page_size as usize)
            .collect();

        Ok(PageResponse::new(list, total, page.page, page.page_size))
    }

    async fn find_all(&self, query: &ConfigQuery) -> Result<Vec<Config>, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .filter(|c| c.audit.deleted == 0)
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

    async fn insert(&self, config: &Config) -> Result<(), RepositoryError> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn update(&self, config: &Config) -> Result<(), RepositoryError> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> Result<(), RepositoryError> {
        let mut configs = self.configs.write().unwrap();
        if let Some(config) = configs.get_mut(&id) {
            config.audit.deleted = 1;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!("Config {} not found", id)))
        }
    }

    async fn exists_by_key(&self, key: &str) -> Result<bool, RepositoryError> {
        let configs = self.configs.read().unwrap();
        Ok(configs
            .values()
            .any(|c| c.config_key == key && c.audit.deleted == 0))
    }
}
