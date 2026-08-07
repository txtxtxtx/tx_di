//! tx_di_nacos — 配置中心 + 服务注册 + 应用启动循环（**非组件 crate**）
//!
//! 不导出任何 `#[derive(Component)]` 结构、不注册进 DI 容器，以普通函数/宏的方式
//! 由应用入口使用。核心价值：把「Nacos 接入」变成三行代码。
//!
//! # 能力
//!
//! 1. **配置中心**：启动时拉取远程配置并与本地 TOML 融合（远程覆盖本地）。
//! 2. **配置变更 → 优雅重启**：监听主配置变更，优雅关闭当前 App（进程不退出），
//!    用新配置重启。
//! 3. **服务注册**：收集 HTTP/gRPC 端点并注册到 Nacos（SDK 自动心跳）。
//!
//! # 快速开始
//!
//! ```rust,ignore
//! #[tokio::main]
//! async fn main() -> AppResult<()> {
//!     tx_di_nacos::app_loop! {
//!         config = r"config/config.toml",
//!         startup = |app: std::sync::Arc<tx_di_core::App>| -> tx_di_core::RIE<()> {
//!             // ins_run 完成后的业务初始化（job handler 注册等）
//!             Ok(())
//!         },
//!     }
//! }
//! ```
//!
//! # 配置（`config.toml` 中的 `[registry_config]`）
//!
//! ```toml
//! [registry_config]
//! enabled = false                # 主开关：true 启用配置中心+服务注册
//! nacos_addr = "http://127.0.0.1:8848"
//! namespace = "public"
//! group = "DEFAULT_GROUP"
//! service_name = "tx-admin"
//! auto_register = true
//! config_data_id = "tx-admin.toml"   # 主配置 data_id（默认 "{service_name}.toml"）
//! ```

pub mod bootstrap;
pub mod client;
pub mod config;
pub mod dynamic_config;
pub mod endpoints;
pub mod model;
pub mod traits;

mod nacos;
mod app_loop;

pub use bootstrap::{load_bootstrap, load_local_toml};
pub use client::NacosClient;
pub use config::RegistryConfig;
pub use dynamic_config::DynamicConfig;
pub use endpoints::{register_endpoints, take_endpoints, EndpointProvider};
pub use model::{Protocol, ServiceEndpoint, ServiceInstance};
pub use traits::{ConfigCenter, ServiceRegistry};
