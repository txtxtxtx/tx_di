//! CAN/CANFD 插件配置（纯配置结构，不进入 DI）

use serde::{Deserialize, Serialize};

/// 适配器类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdapterKind {
    #[serde(alias = "socketcan")]
    SocketCan,
    Pcan,
    Kvaser,
    #[serde(alias = "simbus")]
    SimBus,
}

impl Default for AdapterKind {
    fn default() -> Self {
        AdapterKind::SimBus
    }
}

fn default_adapter() -> AdapterKind {
    AdapterKind::SimBus
}
fn default_interface() -> String {
    "vcan0".to_string()
}
fn default_bitrate() -> u32 {
    500_000
}
fn default_fd_bitrate() -> u32 {
    2_000_000
}
fn default_rx_queue() -> usize {
    512
}
fn default_tx_timeout() -> u64 {
    100
}
fn default_isotp_tx_id() -> u32 {
    0x7E0
}
fn default_isotp_rx_id() -> u32 {
    0x7E8
}
fn default_p2_timeout() -> u64 {
    150
}
fn default_p2_star_timeout() -> u64 {
    5000
}

/// CAN 插件配置
///
/// ```toml
/// [can_config]
/// adapter    = "simbus"
/// interface  = "vcan0"
/// bitrate    = 500_000
/// enable_fd  = false
/// isotp_tx_id = 0x7E0
/// isotp_rx_id = 0x7E8
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanConfig {
    #[serde(default = "default_adapter")]
    pub adapter: AdapterKind,
    #[serde(default = "default_interface")]
    pub interface: String,
    #[serde(default = "default_bitrate")]
    pub bitrate: u32,
    #[serde(default = "default_fd_bitrate")]
    pub fd_bitrate: u32,
    #[serde(default)]
    pub enable_fd: bool,
    #[serde(default = "default_rx_queue")]
    pub rx_queue_size: usize,
    #[serde(default = "default_tx_timeout")]
    pub tx_timeout_ms: u64,
    #[serde(default = "default_isotp_tx_id")]
    pub isotp_tx_id: u32,
    #[serde(default = "default_isotp_rx_id")]
    pub isotp_rx_id: u32,
    #[serde(default)]
    pub isotp_block_size: u8,
    #[serde(default)]
    pub isotp_st_min_ms: u8,
    #[serde(default = "default_p2_timeout")]
    pub uds_p2_timeout_ms: u64,
    #[serde(default = "default_p2_star_timeout")]
    pub uds_p2_star_timeout_ms: u64,
}

impl Default for CanConfig {
    fn default() -> Self {
        CanConfig {
            adapter: default_adapter(),
            interface: default_interface(),
            bitrate: default_bitrate(),
            fd_bitrate: default_fd_bitrate(),
            enable_fd: false,
            rx_queue_size: default_rx_queue(),
            tx_timeout_ms: default_tx_timeout(),
            isotp_tx_id: default_isotp_tx_id(),
            isotp_rx_id: default_isotp_rx_id(),
            isotp_block_size: 0,
            isotp_st_min_ms: 0,
            uds_p2_timeout_ms: default_p2_timeout(),
            uds_p2_star_timeout_ms: default_p2_star_timeout(),
        }
    }
}

impl CanConfig {
    /// 从 TOML 文件加载配置（由 CanPlugin::inner_init 调用）
    pub fn load_from_toml(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let value: toml::Value = toml::from_str(&content)?;
        // 取 [can_config] section
        let table: Option<&toml::map::Map<String, toml::Value>> =
            value.get("can_config").and_then(|v| v.as_table());
        let table = table.ok_or_else(|| {
            anyhow::anyhow!("配置文件中缺少 [can_config] 段")
        })?;
        let config: CanConfig = toml::from_str(&toml::to_string(table)?)?;
        Ok(config)
    }
}
