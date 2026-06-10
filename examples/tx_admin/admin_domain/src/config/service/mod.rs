use std::sync::Arc;

use crate::config::model::aggregate::Config;
use crate::config::model::value_object::ConfigQuery;
use crate::config::repository::ConfigRepository;
use crate::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};
use admin_common::id;

pub struct ConfigService {
    config_repo: Arc<dyn ConfigRepository>,
}

impl ConfigService {
    pub fn new(config_repo: Arc<dyn ConfigRepository>) -> Self {
        Self { config_repo }
    }

    pub async fn create_config(
        &self,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        creator: Option<String>,
    ) -> Result<Config, RepositoryError> {
        if self.config_repo.exists_by_key(&config_key).await? {
            return Err(RepositoryError::Duplicate(format!(
                "Config key '{}' already exists",
                config_key
            )));
        }

        let config_id = id::next_id();
        let config = Config::create(config_id, category, config_type, name, config_key, value, creator);
        self.config_repo.insert(&config).await?;
        Ok(config)
    }

    pub async fn update_config(
        &self,
        config_id: u64,
        category: String,
        config_type: i32,
        name: String,
        config_key: String,
        value: String,
        visible: i32,
        remark: Option<String>,
        updater: Option<String>,
    ) -> Result<Config, RepositoryError> {
        let mut config = self
            .config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Config {} not found", config_id)))?;

        config.update_info(category, config_type, name, config_key, value, visible, remark, updater);
        self.config_repo.update(&config).await?;
        Ok(config)
    }

    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        let mut config = self
            .config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Config {} not found", config_id)))?;

        config.soft_delete(updater);
        self.config_repo.update(&config).await?;
        Ok(())
    }

    pub async fn get_config_page(
        &self,
        query: &ConfigQuery,
        page: &PageRequest,
    ) -> Result<PageResponse<Config>, RepositoryError> {
        self.config_repo.find_page(query, page).await
    }

    pub async fn get_config(&self, config_id: u64) -> Result<Config, RepositoryError> {
        self.config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Config {} not found", config_id)))
    }

    pub async fn get_by_key(&self, key: &str) -> Result<Config, RepositoryError> {
        self.config_repo
            .find_by_key(key)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Config key '{}' not found", key)))
    }
}
