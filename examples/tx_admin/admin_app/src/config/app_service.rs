use std::collections::HashMap;
use std::sync::Arc;
use crate::config::dto::config_to_response;
use admin_domain::config::model::value_object::ConfigQuery;
use admin_domain::config::service::ConfigService;
use admin_proto::{CreateConfigRequest, UpdateConfigRequest, ListConfigsRequest, ConfigResponse};
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

#[tx_comp]
pub struct ConfigAppService {
    config_service: Arc<ConfigService>,
}

impl ConfigAppService {
    /// 创建配置应用服务实例
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self { config_service }
    }

    /// 创建新系统配置
    pub async fn create_config(
        &self,
        req: CreateConfigRequest,
        creator: Option<String>,
    ) -> AppResult<ConfigResponse> {
        let config = self
            .config_service
            .create_config(req.category, req.config_type, req.name, req.config_key, req.value, creator)
            .await?;
        Ok(config_to_response(config))
    }

    /// 更新系统配置
    pub async fn update_config(
        &self,
        req: UpdateConfigRequest,
        updater: Option<String>,
    ) -> AppResult<ConfigResponse> {
        let config = self
            .config_service
            .update_config(
                req.config_id,
                req.category,
                req.config_type,
                req.name,
                req.config_key,
                req.value,
                req.visible,
                req.remark,
                updater,
            )
            .await?;
        Ok(config_to_response(config))
    }

    /// 删除系统配置
    pub async fn delete_config(&self, config_id: u64, updater: Option<String>) -> AppResult<()> {
        self.config_service.delete_config(config_id, updater).await
    }

    /// 分页查询系统配置列表
    pub async fn get_config_page(
        &self,
        req: ListConfigsRequest,
    ) -> AppResult<Page<ConfigResponse>> {
        let query = ConfigQuery {
            name: req.name,
            category: req.category,
            config_key: req.config_key,
            config_type: req.config_type,
        };
        let page = Page::request(req.page, req.page_size);
        let result = self.config_service.get_config_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(config_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }

    /// 根据ID获取配置信息
    pub async fn get_config(&self, config_id: u64) -> AppResult<ConfigResponse> {
        let config = self.config_service.get_config(config_id).await?;
        Ok(config_to_response(config))
    }

    /// 根据配置键获取配置信息
    pub async fn get_by_key(&self, key: &str) -> AppResult<ConfigResponse> {
        let config = self.config_service.get_by_key(key).await?;
        Ok(config_to_response(config))
    }

    /// 批量根据配置键获取配置值
    pub async fn get_by_keys(&self, keys: Vec<String>) -> AppResult<HashMap<String, String>> {
        self.config_service.get_by_keys(&keys).await
    }
}
