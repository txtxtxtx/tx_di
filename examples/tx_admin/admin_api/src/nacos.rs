//! 注册中心端点提供者：把本服务的 HTTP / gRPC 端点注册到服务注册中心（Nacos）
//!
//! `RegistryPlugin`（tx_di_registry）在 `app_async_init`（init_sort = `i32::MAX - 50`）中
//! 收集端点并注册；本模块通过 `register_endpoints` 在 `AdminPlugin`（`i32::MAX - 100`）
//! 中提前提供端点，保证收集顺序正确。

use std::collections::HashMap;

use tx_di_registry::{EndpointProvider, Protocol, ServiceEndpoint};

/// 注册到注册中心的本地端点（HTTP + gRPC）
pub struct AdminEndpoints {
    http_port: u16,
    grpc_port: u16,
    ip: String,
}

impl AdminEndpoints {
    /// 构造端点提供者
    ///
    /// 注册 IP：优先环境变量 `SERVICE_IP`（容器 / 多网卡场景显式指定），
    /// 否则回退 `127.0.0.1`（仅适合单机调试；生产必须通过 `SERVICE_IP` 注入）。
    pub fn new(http_port: u16, grpc_port: u16) -> Self {
        let ip = std::env::var("SERVICE_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
        Self {
            http_port,
            grpc_port,
            ip,
        }
    }
}

impl EndpointProvider for AdminEndpoints {
    fn get_endpoints(&self) -> Vec<ServiceEndpoint> {
        let mut grpc_meta = HashMap::new();
        grpc_meta.insert("protocol".to_string(), "grpc".to_string());
        vec![
            ServiceEndpoint {
                protocol: Protocol::Http,
                ip: self.ip.clone(),
                port: self.http_port,
                metadata: HashMap::new(),
            },
            ServiceEndpoint {
                protocol: Protocol::Grpc,
                ip: self.ip.clone(),
                port: self.grpc_port,
                metadata: grpc_meta,
            },
        ]
    }
}
