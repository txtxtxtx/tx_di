//! tx_di_registry — 服务注册/发现 + 配置中心插件
//!
//! 提供统一的 `ServiceRegistry` 和 `ConfigCenter` trait 抽象，
//! 支持服务注册/发现、配置热更新、双协议（HTTP+gRPC）端点注册。
//!
//! # 快速开始
//!
//! ```toml
//! # Cargo.toml
//! tx_di_registry = { path = "plugins/tx_di_registry" }
//! ```
//!
//! ```toml
//! # configs/registry_config.toml
//! [registry_config]
//! enabled = true
//! nacos_addr = "http://127.0.0.1:8848"
//! namespace = "public"
//! group = "DEFAULT_GROUP"
//! service_name = "my-service"
//! ```

mod config;
mod config_watcher;
mod dynamic_config;
mod endpoints;
mod model;
mod plugin;
mod traits;

#[cfg(feature = "nacos")]
pub mod nacos;

pub use config::RegistryConfig;
pub use dynamic_config::DynamicConfig;
pub use endpoints::{register_endpoints, collect_endpoints, EndpointProvider};
pub use model::{Protocol, ServiceEndpoint, ServiceInstance};
pub use plugin::RegistryPlugin;
pub use traits::{ConfigCenter, ServiceRegistry};
