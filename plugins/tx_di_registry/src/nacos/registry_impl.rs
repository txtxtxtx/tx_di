//! Nacos 服务注册实现

use async_trait::async_trait;
use tx_error::AppResult;

use crate::config::RegistryConfig;
use crate::model::ServiceInstance;
use crate::traits::ServiceRegistry;

/// Nacos 服务注册实现
#[allow(dead_code)]
pub struct NacosServiceRegistry {
    /// Nacos 服务地址
    server_addr: String,
    /// 命名空间
    namespace: String,
}

impl NacosServiceRegistry {
    pub fn new(config: &RegistryConfig) -> Self {
        Self {
            server_addr: config.nacos_addr.clone(),
            namespace: config.namespace.clone(),
        }
    }
}

#[async_trait]
impl ServiceRegistry for NacosServiceRegistry {
    async fn register(&self, instance: &ServiceInstance) -> AppResult<()> {
        // TODO: 使用 nacos_rust_client 注册服务
        tracing::info!(
            service = %instance.service_name,
            instance_id = %instance.instance_id,
            "Nacos 服务注册（TODO: 待接入 nacos_rust_client）"
        );
        Ok(())
    }

    async fn update(&self, instance: &ServiceInstance) -> AppResult<()> {
        tracing::info!(
            service = %instance.service_name,
            instance_id = %instance.instance_id,
            "Nacos 服务更新（TODO）"
        );
        Ok(())
    }

    async fn deregister(&self, instance_id: &str) -> AppResult<()> {
        tracing::info!(instance_id = %instance_id, "Nacos 服务注销（TODO）");
        Ok(())
    }

    async fn discover(&self, service_name: &str) -> AppResult<Vec<ServiceInstance>> {
        tracing::info!(service = %service_name, "Nacos 服务发现（TODO）");
        Ok(Vec::new())
    }

    async fn subscribe(
        &self,
        service_name: &str,
        _callback: Box<dyn Fn(Vec<ServiceInstance>) + Send + Sync>,
    ) {
        tracing::info!(service = %service_name, "Nacos 服务订阅（TODO）");
    }
}
