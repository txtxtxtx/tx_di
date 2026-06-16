use std::collections::HashMap;
use std::sync::Arc;

use crate::config::model::aggregate::Config;
use crate::config::model::value_object::ConfigQuery;
use crate::config::repository::ConfigRepository;
use crate::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::id;

#[tx_comp]
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
    ) -> AppResult<Config> {
        if self.config_repo.exists_by_key(&config_key).await? {
            return Err(RepositoryError::DuplicateConfigKey)?;
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
    ) -> AppResult<Config> {
        let mut config = self
            .config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?;

        config.update_info(category, config_type, name, config_key, value, visible, remark, updater);
        self.config_repo.update(&config).await?;
        Ok(config)
    }

    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> AppResult<()> {
        let mut config = self
            .config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?;

        config.soft_delete(updater);
        self.config_repo.update(&config).await?;
        Ok(())
    }

    pub async fn get_config_page(
        &self,
        query: &ConfigQuery,
        page: Page<Config>,
    ) -> AppResult<Page<Config>> {
        self.config_repo.find_page(query, page).await
    }

    pub async fn get_config(&self, config_id: u64) -> AppResult<Config> {
        Ok(self.config_repo
            .find_by_id(config_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?)
    }

    pub async fn get_by_key(&self, key: &str) -> AppResult<Config> {
        Ok(self.config_repo
            .find_by_key(key)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundConfig)?)
    }

    pub async fn get_by_keys(&self, keys: &[String]) -> AppResult<HashMap<String, String>> {
        let configs = self.config_repo.find_by_keys(keys).await?;
        let map = configs.into_iter()
            .map(|c| (c.config_key, c.value))
            .collect();
        Ok(map)
    }
}

#[cfg(test)]
mod tests;
