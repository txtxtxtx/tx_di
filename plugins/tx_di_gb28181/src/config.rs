//! GB28181 服务端配置

use crate::media::MediaBackendConfig;
use serde::Deserialize;
use std::collections::HashMap;
use tx_di_core::{tx_comp, CompInit};

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
    30000
}
fn default_rtp_end() -> u16 {
    30500
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
/// [gb28181_server_config.media_backend]
/// backend_type = "zlm"
/// [gb28181_server_config.media_backend.zlm]
/// base_url = "http://127.0.0.1:8080"
/// secret = "aaa"
/// timeout_secs = 10
/// rtsp_port = 554
/// rtsps_port = 0
/// rtmp_port = 1935
/// http_port = 8080
/// https_port = 8081
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

    /// 按设备 ID 配置独立密码（优先级高于 auth_password）
    ///
    /// ```toml
    /// [gb28181_server_config.device_passwords]
    /// "34020000001320000001" = "device1_pass"
    /// "34020000001320000002" = "device2_pass"
    /// ```
    #[serde(default)]
    pub device_passwords: HashMap<String, String>,

    /// 媒体层配置
    #[serde(default)]
    pub media: MediaConfig,

    /// 级联配置（上下级平台互联）
    #[serde(default)]
    pub cascade: CascadeConfig,
    
    /// 统一流媒体后端配置（新版）
    ///
    /// 通过 `[gb28181_server_config.media_backend]` 配置，支持 ZLM、MediaMTX、Null。
    /// 若同时配置了 `zlm` 和 `media_backend`，`media_backend` 优先生效。
    #[serde(default)]
    pub media_backend: MediaBackendConfig,
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
            device_passwords: HashMap::new(),
            media: MediaConfig::default(),
            media_backend: MediaBackendConfig::default(),
            cascade: CascadeConfig::default(),
        }
    }
}

impl CompInit for Gb28181ServerConfig {
    fn init_sort() -> i32 {
        10001
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

/// 级联配置（上下级平台互联）
///
/// ```toml
/// [gb28181_server_config.cascade]
/// enable_upper = true
/// enable_lower = false
/// # upper_platform_sip = "sip:192.168.1.1:5060"
/// # upper_platform_id = "34020000002000000001"
/// # upper_auth_password = "12345678"
/// ```
#[derive(Debug, Clone, Deserialize,Default)]
pub struct CascadeConfig {
    /// 本平台是否同时作为上级（接收下级设备/平台注册）
    #[serde(default="enable_upper")]
    pub enable_upper: bool,

    /// 本平台是否同时作为下级（向上级平台注册）
    #[serde(default)]
    pub enable_lower: bool,

    /// 上级平台 SIP 地址（enable_lower=true 时必填）
    pub upper_platform_sip: Option<String>,

    /// 上级平台 ID
    pub upper_platform_id: Option<String>,

    /// 向上级注册的密码
    pub upper_auth_password: Option<String>,
}
fn enable_upper() -> bool {
    true
}
impl Gb28181ServerConfig {
    /// 获取设备认证密码（优先查 device_passwords，fallback 到全局 auth_password）
    pub fn get_password(&self, device_id: &str) -> &str {
        self.device_passwords
            .get(device_id)
            .map(|s| s.as_str())
            .unwrap_or(&self.auth_password)
    }
}
