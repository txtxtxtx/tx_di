//! 注册中心配置

use serde::Deserialize;
use tx_di_core::{Component, RIE, Store};

/// 注册中心配置
///
/// 从 TOML 配置文件 `[registry_config]` 节自动加载。
///
/// ```toml
/// [registry_config]
/// enabled = true
/// nacos_addr = "http://127.0.0.1:8848"
/// namespace = "public"
/// group = "DEFAULT_GROUP"
/// service_name = "my-service"
/// auto_register = true
/// heartbeat_secs = 5
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf, init, init_sort = i32::MIN)]
pub struct RegistryConfig {
    /// 主开关：是否启用注册中心功能
    #[serde(default)]
    pub enabled: bool,

    /// Nacos 服务地址
    #[serde(default = "default_nacos_addr")]
    pub nacos_addr: String,

    /// 命名空间
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// 分组
    #[serde(default = "default_group")]
    pub group: String,

    /// 本地服务名
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// 是否自动注册本地端点
    #[serde(default = "default_true")]
    pub auto_register: bool,

    /// 心跳间隔（秒）
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            nacos_addr: default_nacos_addr(),
            namespace: default_namespace(),
            group: default_group(),
            service_name: default_service_name(),
            auto_register: default_true(),
            heartbeat_secs: default_heartbeat_secs(),
        }
    }
}

fn init(this: &mut RegistryConfig, _store: &Store) -> RIE<()> {
    tracing::info!(
        enabled = this.enabled,
        service_name = %this.service_name,
        nacos_addr = %this.nacos_addr,
        "注册中心配置已加载"
    );
    Ok(())
}

fn default_nacos_addr() -> String { "http://127.0.0.1:8848".into() }
fn default_namespace() -> String { "public".into() }
fn default_group() -> String { "DEFAULT_GROUP".into() }
fn default_service_name() -> String { "unknown-service".into() }
fn default_true() -> bool { true }
fn default_heartbeat_secs() -> u64 { 5 }
