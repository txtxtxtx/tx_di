//! GB28181 设备配置
//!
//! 通过 TOML 配置文件驱动，与 tx-di 框架自动集成。

use serde::Deserialize;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};

/// GB28181 设备参数配置
///
/// 对应 TOML 文件中的 `[gb_config]` 段。
///
/// ```toml
/// [gb_config]
/// device_id      = "34020000001320000001"
/// platform_id    = "34020000002000000001"
/// platform_ip    = "192.168.1.200"
/// platform_port  = 5060
/// realm          = "3402000000"
/// username       = "34020000001320000001"
/// password       = "12345678"
/// heartbeat_secs = 60
/// register_ttl   = 3600
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct Gb28181Config {
    /// GB28181 设备编号，20 位数字字符串
    #[serde(default = "default_device_id")]
    pub device_id: String,

    /// 上级平台设备编号，20 位数字字符串
    #[serde(default = "default_platform_id")]
    pub platform_id: String,

    /// 上级平台 SIP IP 地址
    #[serde(default = "default_platform_ip")]
    pub platform_ip: String,

    /// 上级平台 SIP 端口
    #[serde(default = "default_platform_port")]
    pub platform_port: u16,

    /// SIP 域（realm），通常是平台编号的前 10 位
    #[serde(default = "default_realm")]
    pub realm: String,

    /// 注册用户名（通常等于 device_id）
    #[serde(default = "default_device_id")]
    pub username: String,

    /// 注册密码
    #[serde(default)]
    pub password: String,

    /// 心跳间隔（秒），GB28181 标准建议 60s
    #[serde(default = "default_heartbeat")]
    pub heartbeat_secs: u64,

    /// 注册有效期（秒），标准建议 3600s
    #[serde(default = "default_ttl")]
    pub register_ttl: u32,

    /// 本机 SIP 地址（用于构造 Contact/From）
    ///
    /// 若不填，自动从 sip_config.host 读取
    pub local_ip: Option<String>,

    /// 本机 SIP 端口（若不填则从 sip_config.port 读取）
    pub local_port: Option<u16>,
}

impl Default for Gb28181Config {
    fn default() -> Self {
        Self {
            device_id: default_device_id(),
            platform_id: default_platform_id(),
            platform_ip: default_platform_ip(),
            platform_port: default_platform_port(),
            realm: default_realm(),
            username: default_device_id(),
            password: String::new(),
            heartbeat_secs: default_heartbeat(),
            register_ttl: default_ttl(),
            local_ip: None,
            local_port: None,
        }
    }
}

impl CompInit for Gb28181Config {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        Ok(())
    }
    fn init_sort() -> i32 {
        i32::MAX - 3
    }
}

impl Gb28181Config {
    /// 构造上级平台 SIP URI，例如 `sip:34020000002000000001@192.168.1.200:5060`
    pub fn platform_uri(&self) -> String {
        format!(
            "sip:{}@{}:{}",
            self.platform_id, self.platform_ip, self.platform_port
        )
    }

    /// 构造本机 Contact URI，例如 `sip:34020000001320000001@192.168.1.100:5060`
    ///
    /// 需要传入从 SipConfig 中读取的 IP/Port（若 gb_config 没有单独指定的话）。
    pub fn contact_uri(&self, fallback_ip: &str, fallback_port: u16) -> String {
        let ip = self.local_ip.as_deref().unwrap_or(fallback_ip);
        let port = self.local_port.unwrap_or(fallback_port);
        format!("sip:{}@{}:{}", self.device_id, ip, port)
    }

    /// 心跳 XML 消息体（GB28181 Keepalive 规范格式）
    pub fn keepalive_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="GB2312"?>
<Notify>
<CmdType>Keepalive</CmdType>
<SN>{}</SN>
<DeviceID>{}</DeviceID>
<Status>OK</Status>
</Notify>"#,
            // SN：序列号，每次自增；这里用 unix timestamp 简化
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            self.device_id
        )
    }
}

fn default_device_id() -> String {
    "34020000001320000001".to_string()
}
fn default_platform_id() -> String {
    "34020000002000000001".to_string()
}
fn default_platform_ip() -> String {
    "192.168.1.200".to_string()
}
fn default_platform_port() -> u16 {
    5060
}
fn default_realm() -> String {
    "3402000000".to_string()
}
fn default_heartbeat() -> u64 {
    60
}
fn default_ttl() -> u32 {
    3600
}
