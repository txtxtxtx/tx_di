//! Nacos 配置中心实现

use async_trait::async_trait;
use tx_error::AppResult;

use crate::config::RegistryConfig;
use crate::traits::ConfigCenter;

/// Nacos 配置中心实现
#[allow(dead_code)]
pub struct NacosConfigCenter {
    server_addr: String,
    namespace: String,
}

impl NacosConfigCenter {
    pub fn new(config: &RegistryConfig) -> Self {
        Self {
            server_addr: config.nacos_addr.clone(),
            namespace: config.namespace.clone(),
        }
    }
}

#[async_trait]
impl ConfigCenter for NacosConfigCenter {
    async fn get_config(
        &self,
        data_id: &str,
        group: &str,
    ) -> AppResult<Option<String>> {
        tracing::info!(
            data_id = %data_id,
            group = %group,
            "Nacos 获取配置（TODO: 待接入 nacos_rust_client）"
        );
        Ok(None)
    }

    async fn publish_config(
        &self,
        data_id: &str,
        group: &str,
        _content: &str,
    ) -> AppResult<()> {
        tracing::info!(
            data_id = %data_id,
            group = %group,
            "Nacos 发布配置（TODO）"
        );
        Ok(())
    }

    async fn remove_config(&self, data_id: &str, group: &str) -> AppResult<()> {
        tracing::info!(data_id = %data_id, group = %group, "Nacos 删除配置（TODO）");
        Ok(())
    }

    async fn listen_config(
        &self,
        data_id: &str,
        group: &str,
        _callback: Box<dyn Fn(String) + Send + Sync>,
    ) {
        tracing::info!(
            data_id = %data_id,
            group = %group,
            "Nacos 监听配置（TODO: 待接入 nacos_rust_client 长轮询）"
        );
        // TODO: 启动 nacos_rust_client 的长轮询监听
        // 当前只是阻塞等待
        std::future::pending::<()>().await;
    }
}
