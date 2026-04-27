//! GB28181 设备客户端配置

use serde::Deserialize;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};

/// GB28181 设备客户端配置
///
/// ```toml
/// [gb28181_device_config]
/// device_id      = "34020000001320000001"
/// platform_ip    = "192.168.1.200"
/// platform_port  = 5060
/// platform_id    = "34020000002000000001"
/// realm          = "3402000000"
/// username       = "34020000001320000001"
/// password       = "12345678"
/// local_ip       = "192.168.1.100"
/// local_port     = 5060
/// heartbeat_secs = 60
/// register_ttl   = 3600
/// rtp_port       = 10000
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct Gb28181DeviceConfig {
    /// 设备 ID（20 位，GB28181 规范）
    pub device_id: String,

    /// 上级平台 IP
    pub platform_ip: String,

    /// 上级平台 SIP 端口
    #[serde(default = "default_platform_port")]
    pub platform_port: u16,

    /// 上级平台 ID
    pub platform_id: String,

    /// 认证域
    #[serde(default = "default_realm")]
    pub realm: String,

    /// 注册用户名（通常与 device_id 相同）
    pub username: String,

    /// 注册密码
    pub password: String,

    /// 本机 SIP IP（用于 From/Contact 头）
    pub local_ip: String,

    /// 本机 SIP 端口（用于 From/Contact 头）
    #[serde(default = "default_local_port")]
    pub local_port: u16,

    /// 心跳间隔（秒），GB28181 建议 60s
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,

    /// 注册有效期（秒）
    #[serde(default = "default_register_ttl")]
    pub register_ttl: u32,

    /// 设备推流 RTP 端口
    #[serde(default = "default_rtp_port")]
    pub rtp_port: u16,

    /// 注册失败最大重试次数（0 = 无限重试）
    #[serde(default = "default_max_retries")]
    pub max_register_retries: u32,

    /// 重试初始间隔（秒）
    #[serde(default = "default_retry_interval")]
    pub retry_interval_secs: u64,
}

impl Gb28181DeviceConfig {
    /// 获取上级平台 SIP URI
    pub fn platform_uri(&self) -> String {
        format!("sip:{}@{}:{}", self.platform_id, self.platform_ip, self.platform_port)
    }

    /// 获取设备本机 Contact URI
    pub fn contact_uri(&self) -> String {
        format!("sip:{}@{}:{}", self.device_id, self.local_ip, self.local_port)
    }
}

impl CompInit for Gb28181DeviceConfig {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        Ok(())
    }
    fn init_sort() -> i32 {
        i32::MAX - 3
    }
}

fn default_platform_port() -> u16 { 5060 }
fn default_realm() -> String { "3402000000".to_string() }
fn default_local_port() -> u16 { 5060 }
fn default_heartbeat_secs() -> u64 { 60 }
fn default_register_ttl() -> u32 { 3600 }
fn default_rtp_port() -> u16 { 10000 }
fn default_max_retries() -> u32 { 0 }
fn default_retry_interval() -> u64 { 5 }
