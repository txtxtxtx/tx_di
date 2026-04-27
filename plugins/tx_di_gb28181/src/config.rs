//! GB28181 服务端配置

use serde::Deserialize;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};

/// 媒体层配置（RTP/RTSP 推流参数）
#[derive(Debug, Clone, Deserialize)]
pub struct MediaConfig {
    /// 媒体服务器本机 IP（用于 SDP c= 行）
    #[serde(default = "default_media_ip")]
    pub local_ip: String,

    /// RTP 端口起始值（动态分配时从此端口开始）
    #[serde(default = "default_rtp_start")]
    pub rtp_port_start: u16,

    /// RTP 端口结束值
    #[serde(default = "default_rtp_end")]
    pub rtp_port_end: u16,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            local_ip: default_media_ip(),
            rtp_port_start: default_rtp_start(),
            rtp_port_end: default_rtp_end(),
        }
    }
}

fn default_media_ip() -> String {
    "0.0.0.0".to_string()
}
fn default_rtp_start() -> u16 {
    10000
}
fn default_rtp_end() -> u16 {
    20000
}

/// GB28181 上级平台服务端配置
///
/// ```toml
/// [gb28181_server_config]
/// platform_id     = "34020000002000000001"
/// realm           = "3402000000"
/// heartbeat_timeout_secs = 120
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
pub struct Gb28181ServerConfig {
    /// 本平台 ID（20 位编号，GB28181 规范）
    #[serde(default = "default_platform_id")]
    pub platform_id: String,

    /// 认证域（realm），用于 SIP 摘要认证
    #[serde(default = "default_realm")]
    pub realm: String,

    /// 设备心跳超时时间（秒）；超时未收到心跳则视为离线
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_secs: u64,

    /// 设备注册有效期上限（秒），回写到 200 OK 的 Contact Expires
    #[serde(default = "default_register_ttl")]
    pub register_ttl: u32,

    /// 是否开启摘要认证（生产环境建议开启）
    #[serde(default)]
    pub enable_auth: bool,

    /// 认证密码（简化：所有设备共用；生产可替换为按 device_id 查库）
    #[serde(default = "default_auth_password")]
    pub auth_password: String,

    /// 媒体层配置
    #[serde(default)]
    pub media: MediaConfig,
}

impl Default for Gb28181ServerConfig {
    fn default() -> Self {
        Self {
            platform_id: default_platform_id(),
            realm: default_realm(),
            heartbeat_timeout_secs: default_heartbeat_timeout(),
            register_ttl: default_register_ttl(),
            enable_auth: false,
            auth_password: default_auth_password(),
            media: MediaConfig::default(),
        }
    }
}

impl CompInit for Gb28181ServerConfig {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        Ok(())
    }
    fn init_sort() -> i32 {
        i32::MAX - 3
    }
}

fn default_platform_id() -> String {
    "34020000002000000001".to_string()
}
fn default_realm() -> String {
    "3402000000".to_string()
}
fn default_heartbeat_timeout() -> u64 {
    120
}
fn default_register_ttl() -> u32 {
    3600
}
fn default_auth_password() -> String {
    "12345678".to_string()
}
