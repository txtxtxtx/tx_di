use std::collections::HashMap;
use std::sync::Arc;
use crate::config::dto::*;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::service::ConfigService;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct ConfigAppService {
    config_service: Arc<ConfigService>,
}

impl ConfigAppService {
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self { config_service }
    }

    pub async fn create_config(
        &self,
        cmd: CreateConfigCommand,
        creator: Option<String>,
    ) -> AppResult<ConfigResponse> {
        let config = self
            .config_service
            .create_config(cmd.category, cmd.config_type, cmd.name, cmd.config_key, cmd.value, creator)
            .await?;
        Ok(ConfigResponse::from(config))
    }

    pub async fn update_config(
        &self,
        cmd: UpdateConfigCommand,
        updater: Option<String>,
    ) -> AppResult<ConfigResponse> {
        let config = self
            .config_service
            .update_config(
                cmd.config_id,
                cmd.category,
                cmd.config_type,
                cmd.name,
                cmd.config_key,
                cmd.value,
                cmd.visible,
                cmd.remark,
                updater,
            )
            .await?;
        Ok(ConfigResponse::from(config))
    }

    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> AppResult<()> {
        self.config_service.delete_config(config_id, updater).await
    }

    pub async fn get_config_page(
        &self,
        request: ConfigQueryRequest,
    ) -> AppResult<Page<ConfigResponse>> {
        let query = ConfigQuery {
            name: request.name,
            category: request.category,
            config_key: request.config_key,
            config_type: request.config_type,
        };
        let page = Page::request(request.page, request.size);
        let result = self.config_service.get_config_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(ConfigResponse::from).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    pub async fn get_config(&self, config_id: u64) -> AppResult<ConfigResponse> {
        let config = self.config_service.get_config(config_id).await?;
        Ok(ConfigResponse::from(config))
    }

    pub async fn get_by_key(&self, key: &str) -> AppResult<ConfigResponse> {
        let config = self.config_service.get_by_key(key).await?;
        Ok(ConfigResponse::from(config))
    }

    pub async fn get_by_keys(&self, keys: Vec<String>) -> AppResult<HashMap<String, String>> {
        self.config_service.get_by_keys(&keys).await
    }
}
