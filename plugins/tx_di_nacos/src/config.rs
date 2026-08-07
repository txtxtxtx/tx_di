//! Nacos 客户端配置（非 DI 组件，仅供 bootstrap 读取 `[registry_config]`）

use serde::Deserialize;

/// Nacos 客户端配置
///
/// 从本地 TOML 配置文件 `[registry_config]` 节加载（bootstrap）。
///
/// ```toml
/// [registry_config]
/// enabled = false
/// nacos_addr = "http://127.0.0.1:8848"
/// namespace = "public"
/// group = "DEFAULT_GROUP"
/// service_name = "tx-admin"
/// auto_register = true
/// config_data_id = "tx-admin.toml"
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryConfig {
    /// 主开关：是否启用 Nacos（配置中心 + 服务注册）
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

    /// 本地服务名（注册到 Nacos 的 service name）
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// 是否自动注册本地端点
    #[serde(default = "default_true")]
    pub auto_register: bool,

    /// 心跳间隔（秒，nacos-sdk 自动心跳，此值仅作记录）
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,

    /// 主配置 data_id（整份 TOML，远程覆盖本地；默认 `"{service_name}.toml"`）
    #[serde(default)]
    pub config_data_id: Option<String>,
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
            config_data_id: None,
        }
    }
}

impl RegistryConfig {
    /// 主配置 data_id（默认 `"{service_name}.toml"`）
    pub fn config_data_id(&self) -> String {
        self.config_data_id
            .clone()
            .unwrap_or_else(|| format!("{}.toml", self.service_name))
    }
}

fn default_nacos_addr() -> String {
    "http://127.0.0.1:8848".into()
}
fn default_namespace() -> String {
    "public".into()
}
fn default_group() -> String {
    "DEFAULT_GROUP".into()
}
fn default_service_name() -> String {
    "unknown-service".into()
}
fn default_true() -> bool {
    true
}
fn default_heartbeat_secs() -> u64 {
    5
}
