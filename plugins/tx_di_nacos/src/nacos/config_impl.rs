//! Nacos 配置中心实现（官方 nacos-group/nacos-sdk-rust）
//!
//! 依赖 nacos-sdk 的 gRPC 双工长连接（Nacos 2.x 协议）：
//! - `get_config`：配置不存在时返回 `None`（`ConfigNotFound` 归一化）
//! - `publish/remove_config`：发布与删除
//! - `listen_config`：订阅配置变更（SDK 后台驱动），阻塞直到取消

use std::sync::Arc;

use async_trait::async_trait;
use nacos_sdk::api::config::{
    ConfigChangeListener, ConfigResponse, ConfigService, ConfigServiceBuilder,
};
use nacos_sdk::api::error::Error as NacosError;
use nacos_sdk::api::props::ClientProps;
use tx_error::{AppError, AppResult};

use crate::config::RegistryConfig;
use crate::traits::ConfigCenter;

use super::registry_impl::normalize_addr;

/// Nacos 配置中心实现（官方 nacos-sdk）
pub struct NacosConfigCenter {
    client: ConfigService,
}

impl NacosConfigCenter {
    /// 构建 Nacos 配置中心客户端
    pub async fn new(config: &RegistryConfig) -> AppResult<Self> {
        let mut props = ClientProps::new()
            .server_addr(normalize_addr(&config.nacos_addr))
            .namespace(config.namespace.clone())
            .app_name("tx_di_nacos")
            .load_cache_at_start(true);
        if let Some(u) = &config.username {
            props = props.auth_username(u.clone());
        }
        if let Some(p) = &config.password {
            props = props.auth_password(p.clone());
        }
        let client = ConfigServiceBuilder::new(props)
            .build()
            .await
            .map_err(|e| AppError::from(format!("Nacos 配置中心客户端构建失败: {e}")))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ConfigCenter for NacosConfigCenter {
    async fn get_config(&self, data_id: &str, group: &str) -> AppResult<Option<String>> {
        match self
            .client
            .get_config(data_id.to_string(), group.to_string())
            .await
        {
            Ok(resp) => Ok(Some(resp.content().clone())),
            // 配置不存在 → None（业务语义：配置缺失按空处理）
            Err(NacosError::ConfigNotFound(_)) => Ok(None),
            Err(e) => Err(AppError::from(format!("Nacos 获取配置失败: {e}"))),
        }
    }

    async fn publish_config(&self, data_id: &str, group: &str, content: &str) -> AppResult<()> {
        self.client
            .publish_config(
                data_id.to_string(),
                group.to_string(),
                content.to_string(),
                None,
            )
            .await
            .map_err(|e| AppError::from(format!("Nacos 发布配置失败: {e}")))?;
        Ok(())
    }

    async fn remove_config(&self, data_id: &str, group: &str) -> AppResult<()> {
        self.client
            .remove_config(data_id.to_string(), group.to_string())
            .await
            .map_err(|e| AppError::from(format!("Nacos 删除配置失败: {e}")))?;
        Ok(())
    }

    async fn listen_config(
        &self,
        data_id: &str,
        group: &str,
        callback: Box<dyn Fn(String) + Send + Sync>,
    ) {
        // 订阅配置变更（SDK 后台驱动），随后阻塞直到取消
        let listener = Arc::new(CallbackConfigListener { callback });
        let _ = self
            .client
            .add_listener(data_id.to_string(), group.to_string(), listener)
            .await;
        std::future::pending::<()>().await;
    }
}

/// 配置变更监听器：变更时以新内容回调
struct CallbackConfigListener {
    callback: Box<dyn Fn(String) + Send + Sync>,
}

impl ConfigChangeListener for CallbackConfigListener {
    fn notify(&self, config_resp: ConfigResponse) {
        (self.callback)(config_resp.content().clone());
    }
}
