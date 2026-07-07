//! GB28181 服务端配置

use crate::media::MediaBackendConfig;
use serde::Deserialize;
use std::collections::HashMap;
use tx_di_core::Component;
use tx_gb28181::GbVersion;

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

    /// NAT 穿透时对外暴露的公网 IP（用于 SDP `c=` 行与广播 `<IP>` 字段）。
    ///
    /// 不填（默认）时使用 `local_ip` / `sip_ip`；填写后所有**出网** SDP 媒体地址
    /// 与该公网 IP 对齐，解决服务器在私有网络、对端无法回包的问题。
    #[serde(default)]
    pub nat_external_ip: Option<String>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            local_ip: default_media_ip(),
            rtp_port_start: default_rtp_start(),
            rtp_port_end: default_rtp_end(),
            nat_external_ip: None,
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
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf)]
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

    // ── 访问控制 ──────────────────────────────────────────────────────────────

    /// 设备注册白名单（device_id 列表）
    ///
    /// 非空时：**仅白名单中的设备允许注册**，其余设备返回 403 Forbidden。
    /// 空列表（默认）：不启用白名单，所有设备均可注册（但仍受黑名单限制）。
    ///
    /// ```toml
    /// [gb28181_server_config]
    /// allowed_device_ids = ["34020000001320000001", "34020000001320000002"]
    /// ```
    #[serde(default)]
    pub allowed_device_ids: Vec<String>,

    /// 设备注册黑名单（device_id 列表）
    ///
    /// 黑名单中的设备**始终被拒绝注册**（403 Forbidden），优先级高于白名单。
    ///
    /// ```toml
    /// [gb28181_server_config]
    /// blocked_device_ids = ["34020000001990000099"]
    /// ```
    #[serde(default)]
    pub blocked_device_ids: Vec<String>,

    // ── 协议版本（每设备粒度）─────────────────────────────────────────────────

    /// 默认协议版本（新注册设备未在下表命中时使用），默认 2022
    ///
    /// ```toml
    /// [gb28181_server_config]
    /// default_version = "2022"
    /// ```
    #[serde(default)]
    pub default_version: GbVersion,

    /// 按设备 ID 覆盖协议版本（混合组网：部分 2016 老设备 + 2022 新设备）
    ///
    /// ```toml
    /// [gb28181_server_config.device_versions]
    /// "34020000001320000001" = "2016"
    /// "34020000001320000002" = "2022"
    /// ```
    #[serde(default)]
    pub device_versions: HashMap<String, GbVersion>,
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
            allowed_device_ids: Vec::new(),
            blocked_device_ids: Vec::new(),
            default_version: GbVersion::default(),
            device_versions: HashMap::new(),
        }
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

    /// 上级平台协议版本（决定出网目录 XML 的字符集：2016→GB2312，2022→GB18030）
    #[serde(default)]
    pub upper_version: GbVersion,
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

    /// 检查设备是否允许注册（ACL 白名单/黑名单）
    ///
    /// 返回 `Ok(())` 表示允许，`Err(reason)` 表示拒绝。
    ///
    /// 规则优先级：
    /// 1. 黑名单命中 → 拒绝（黑名单优先）
    /// 2. 白名单非空且未命中 → 拒绝
    /// 3. 其余情况 → 允许
    /// 解析指定设备的协议版本（每设备覆盖优先，回退默认版本）
    ///
    /// 混合组网场景下，部分老设备为 2016、新设备为 2022，平台据此
    /// 决定下发 XML 的字符集（GB2312 / GB18030）与可下发的指令集。
    pub fn device_version_for(&self, device_id: &str) -> GbVersion {
        self.device_versions
            .get(device_id)
            .copied()
            .unwrap_or(self.default_version)
    }

    pub fn check_device_allowed(&self, device_id: &str) -> Result<(), String> {
        // 黑名单优先
        if self.blocked_device_ids.iter().any(|id| id == device_id) {
            return Err(format!("设备 {device_id} 在黑名单中"));
        }
        // 白名单非空时，必须在白名单中
        if !self.allowed_device_ids.is_empty()
            && !self.allowed_device_ids.iter().any(|id| id == device_id)
        {
            return Err(format!("设备 {device_id} 不在白名单中"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_fallback_to_default() {
        let cfg = Gb28181ServerConfig::default();
        assert_eq!(cfg.device_version_for("unknown-dev"), GbVersion::V2022);
    }

    #[test]
    fn version_per_device_override() {
        let mut cfg = Gb28181ServerConfig::default();
        cfg.default_version = GbVersion::V2022;
        cfg.device_versions
            .insert("34020000001320000001".to_string(), GbVersion::V2016);
        assert_eq!(
            cfg.device_version_for("34020000001320000001"),
            GbVersion::V2016
        );
        // 未命中覆盖的仍用默认
        assert_eq!(cfg.device_version_for("other"), GbVersion::V2022);
    }

    #[test]
    fn version_default_is_v2022() {
        let cfg = Gb28181ServerConfig::default();
        assert_eq!(cfg.default_version, GbVersion::V2022);
    }
}

