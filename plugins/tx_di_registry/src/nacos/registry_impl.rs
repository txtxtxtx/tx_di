//! Nacos 服务注册/发现实现（官方 nacos-group/nacos-sdk-rust）
//!
//! 依赖 nacos-sdk 的 gRPC 双工长连接（Nacos 2.x 协议）：
//! - `register`：一个服务下按端点批量注册实例，SDK 自动心跳保活
//! - `discover`：拉取实例并按 ip 聚合为多协议端点
//! - `subscribe`：订阅服务变更（SDK 后台驱动，事件回调）

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nacos_sdk::api::naming::{
    NamingChangeEvent, NamingEventListener, NamingService, NamingServiceBuilder,
    ServiceInstance as NacosInstance,
};
use nacos_sdk::api::props::ClientProps;
use tx_error::{AppError, AppResult};

use crate::config::RegistryConfig;
use crate::model::{Protocol, ServiceEndpoint, ServiceInstance};
use crate::traits::ServiceRegistry;

/// 已注册实例记录（deregister/update 用）
struct RegisteredRecord {
    service_name: String,
    group: String,
    instances: Vec<NacosInstance>,
}

/// Nacos 服务注册/发现实现（官方 nacos-sdk）
pub struct NacosServiceRegistry {
    client: NamingService,
    group: String,
    /// instance_id → 已注册实例
    registered: Mutex<HashMap<String, RegisteredRecord>>,
}

impl NacosServiceRegistry {
    /// 构建 Nacos 服务注册客户端（gRPC 双工长连接，自动重连）
    pub async fn new(config: &RegistryConfig) -> AppResult<Self> {
        let props = ClientProps::new()
            .server_addr(normalize_addr(&config.nacos_addr))
            .namespace(config.namespace.clone())
            .app_name("tx_di_registry")
            .load_cache_at_start(true);
        let client = NamingServiceBuilder::new(props)
            .build()
            .await
            .map_err(|e| AppError::from(format!("Nacos 注册中心客户端构建失败: {e}")))?;
        Ok(Self {
            client,
            group: config.group.clone(),
            registered: Mutex::new(HashMap::new()),
        })
    }

    /// 将 tx_di 端点转为一个 Nacos ServiceInstance（每个端点一个实例，自动心跳）
    fn to_nacos_instances(instance: &ServiceInstance) -> Vec<NacosInstance> {
        instance
            .endpoints
            .iter()
            .map(|ep| NacosInstance {
                ip: ep.ip.clone(),
                port: ep.port as i32,
                weight: 1.0,
                healthy: instance.healthy,
                enabled: true,
                metadata: ep.metadata.clone(),
                ..Default::default()
            })
            .collect()
    }

    /// 将 Nacos 实例列表按 ip 聚合成 tx_di ServiceInstance（多协议端点合并到同实例）
    fn to_service_instances(
        service_name: &str,
        instances: Vec<NacosInstance>,
    ) -> Vec<ServiceInstance> {
        let mut map: HashMap<String, ServiceInstance> = HashMap::new();
        for ni in instances {
            let ip = ni.ip.clone();
            let entry = map.entry(ip.clone()).or_insert_with(|| ServiceInstance {
                service_name: service_name.to_string(),
                instance_id: ni
                    .instance_id
                    .clone()
                    .unwrap_or_else(|| format!("{}-{}", service_name, ip)),
                endpoints: Vec::new(),
                healthy: ni.healthy,
                metadata: ni.metadata.clone(),
            });
            let metadata = ni.metadata.clone();
            let protocol = if metadata
                .get("protocol")
                .is_some_and(|p| p.eq_ignore_ascii_case("grpc"))
            {
                Protocol::Grpc
            } else {
                Protocol::Http
            };
            entry.endpoints.push(ServiceEndpoint {
                protocol,
                ip: ip.clone(),
                port: ni.port as u16,
                metadata,
            });
        }
        map.into_values().collect()
    }
}

#[async_trait]
impl ServiceRegistry for NacosServiceRegistry {
    async fn register(&self, instance: &ServiceInstance) -> AppResult<()> {
        let nacos_instances = Self::to_nacos_instances(instance);
        if nacos_instances.is_empty() {
            return Err(AppError::from("注册实例无可用端点"));
        }
        // 批量注册（每个端点一个实例，SDK 自动心跳保活）
        self.client
            .batch_register_instance(
                instance.service_name.clone(),
                Some(self.group.clone()),
                nacos_instances.clone(),
            )
            .await
            .map_err(|e| AppError::from(format!("Nacos 注册失败: {e}")))?;

        self.registered.lock().unwrap().insert(
            instance.instance_id.clone(),
            RegisteredRecord {
                service_name: instance.service_name.clone(),
                group: self.group.clone(),
                instances: nacos_instances,
            },
        );
        Ok(())
    }

    async fn update(&self, instance: &ServiceInstance) -> AppResult<()> {
        // Nacos 更新实例 = 注销旧 + 注册新（元数据变更）
        self.deregister(&instance.instance_id).await?;
        self.register(instance).await
    }

    async fn deregister(&self, instance_id: &str) -> AppResult<()> {
        let record = self.registered.lock().unwrap().remove(instance_id);
        if let Some(record) = record {
            for ni in record.instances {
                self.client
                    .deregister_instance(
                        record.service_name.clone(),
                        Some(record.group.clone()),
                        ni,
                    )
                    .await
                    .map_err(|e| AppError::from(format!("Nacos 注销失败: {e}")))?;
            }
        }
        Ok(())
    }

    async fn discover(&self, service_name: &str) -> AppResult<Vec<ServiceInstance>> {
        let instances = self
            .client
            .get_all_instances(service_name.to_string(), Some(self.group.clone()), Vec::new(), true)
            .await
            .map_err(|e| AppError::from(format!("Nacos 服务发现失败: {e}")))?;
        Ok(Self::to_service_instances(service_name, instances))
    }

    async fn subscribe(
        &self,
        service_name: &str,
        callback: Box<dyn Fn(Vec<ServiceInstance>) + Send + Sync>,
    ) {
        let listener = Arc::new(NacosEventListener {
            service_name: service_name.to_string(),
            callback,
        });
        let _ = self
            .client
            .subscribe(
                service_name.to_string(),
                Some(self.group.clone()),
                Vec::new(),
                listener,
            )
            .await;
    }
}

/// 服务变更监听器（SDK 后台驱动，变更时回调）
struct NacosEventListener {
    service_name: String,
    callback: Box<dyn Fn(Vec<ServiceInstance>) + Send + Sync>,
}

impl NamingEventListener for NacosEventListener {
    fn event(&self, event: Arc<NamingChangeEvent>) {
        if let Some(instances) = &event.instances {
            let list =
                NacosServiceRegistry::to_service_instances(&self.service_name, instances.clone());
            (self.callback)(list);
        }
    }
}

/// 规范化 Nacos 地址：去掉 `http://`/`https://` 前缀与末尾 `/`，仅保留 `ip:port`
pub(crate) fn normalize_addr(addr: &str) -> String {
    addr.trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string()
}
