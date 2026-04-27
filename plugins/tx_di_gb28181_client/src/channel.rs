//! 设备通道配置

use serde::Deserialize;

/// 设备通道描述（用于目录上报）
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelConfig {
    /// 通道 ID（20 位）
    pub channel_id: String,
    /// 通道名称
    pub name: String,
    /// 厂商
    #[serde(default = "default_manufacturer")]
    pub manufacturer: String,
    /// 型号
    #[serde(default = "default_model")]
    pub model: String,
    /// 通道状态（"ON" | "OFF"）
    #[serde(default = "default_status")]
    pub status: String,
}

impl ChannelConfig {
    pub fn new(channel_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
            name: name.into(),
            manufacturer: default_manufacturer(),
            model: default_model(),
            status: default_status(),
        }
    }
}

fn default_manufacturer() -> String { "Simulator".to_string() }
fn default_model() -> String { "IPC-V1".to_string() }
fn default_status() -> String { "ON".to_string() }
