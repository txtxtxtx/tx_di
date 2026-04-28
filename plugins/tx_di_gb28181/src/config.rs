//! GB28181 服务端配置

use crate::zlm::ZlmConfig;
use serde::Deserialize;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};

/// 媒体层配置（RTP/RTSP 推流参数）
#[derive(Debug, Clone, Deserialize)]
pub struct MediaConfig {
    /// 媒体服务器本机 IP（用于 SDP c= 行，0.0.0.0 表示自动探测）
    #[serde(default = "default_media_ip")]
    pub local_ip: String,

    /// RTP 端口起始值（动态分配时从此端口开始，0 表示由 ZLM 自动分配）
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
/// platform_id            = "34020000002000000001"
/// realm                  = "3402000000"
/// sip_ip                 = "192.168.1.100"
/// heartbeat_timeout_secs = 120
/// enable_auth            = true
/// auth_password          = "12345678"
///
/// [gb28181_server_config.media]
/// local_ip       = "192.168.1.100"
/// rtp_port_start = 10000
/// rtp_port_end   = 20000
///
/// [gb28181_server_config.zlm]
/// base_url = "http://127.0.0.1:8080"
/// secret   = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
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

    /// 平台 SIP 服务对外暴露的 IP（用于构造 SIP URI）
    #[serde(default = "default_sip_ip")]
    pub sip_ip: String,

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

    /// ZLMediaServer 配置
    #[serde(default)]
    pub zlm: ZlmConfig,
}

impl Default for Gb28181ServerConfig {
    fn default() -> Self {
        Self {
            platform_id: default_platform_id(),
            realm: default_realm(),
            sip_ip: default_sip_ip(),
            heartbeat_timeout_secs: default_heartbeat_timeout(),
            register_ttl: default_register_ttl(),
            enable_auth: false,
            auth_password: default_auth_password(),
            media: MediaConfig::default(),
            zlm: ZlmConfig::default(),
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
fn default_sip_ip() -> String {
    "127.0.0.1".to_string()
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
