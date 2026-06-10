use std::sync::Arc;

use crate::config::dto::*;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::service::ConfigService;
use admin_domain::shared::repository::RepositoryError;
use admin_common::types::{PageRequest, PageResponse};

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
    ) -> Result<ConfigResponse, RepositoryError> {
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
    ) -> Result<ConfigResponse, RepositoryError> {
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

    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> Result<(), RepositoryError> {
        self.config_service.delete_config(config_id, updater).await
    }

    pub async fn get_config_page(
        &self,
        request: ConfigQueryRequest,
    ) -> Result<PageResponse<ConfigResponse>, RepositoryError> {
        let query = ConfigQuery {
            name: request.name,
            category: request.category,
            config_key: request.config_key,
            config_type: request.config_type,
        };
        let page = PageRequest::new(request.page, request.page_size);
        let result = self.config_service.get_config_page(&query, &page).await?;

        Ok(PageResponse::new(
            result.list.into_iter().map(ConfigResponse::from).collect(),
            result.total,
            result.page,
            result.page_size,
        ))
    }

    pub async fn get_config(&self, config_id: u64) -> Result<ConfigResponse, RepositoryError> {
        let config = self.config_service.get_config(config_id).await?;
        Ok(ConfigResponse::from(config))
    }

    pub async fn get_by_key(&self, key: &str) -> Result<ConfigResponse, RepositoryError> {
        let config = self.config_service.get_by_key(key).await?;
        Ok(ConfigResponse::from(config))
    }
}
