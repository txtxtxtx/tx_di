//! 设备端配置

use serde::Deserialize;
use tx_di_core::Component;
use tx_gb28181::GbVersion;

/// 设备端配置（TOML `[gb_dev]`）
///
/// 所有字段均有 `#[serde(default)]`，配置段缺失时退化为全默认
/// （`enabled = false`），不会破坏未使用本组件的应用构建。
#[derive(Debug, Clone, Default, Deserialize, Component)]
#[component(conf = "gb_dev")]
pub struct GbDevConfig {
    /// 上级平台地址（完整 SIP URI，如 `"sip:34020000002000000001@192.168.1.1:5060"`）
    ///
    /// 同时作为：REGISTER 的注册服务器地址（取 `@` 之后部分）与出网 MESSAGE 的目标 URI。
    #[serde(default)]
    pub platform_uri: String,

    /// 本设备 ID（注册用户名，通常为 20 位编码）
    #[serde(default)]
    pub device_id: String,

    /// 注册用户名（一般等于 device_id）
    #[serde(default)]
    pub username: String,

    /// 注册密码
    #[serde(default)]
    pub password: String,

    /// 认证域（realm）；`None` 表示接受任意挑战
    #[serde(default)]
    pub realm: Option<String>,

    /// 注册有效期（秒），默认 3600
    #[serde(default = "default_ttl")]
    pub register_ttl: u32,

    /// 心跳间隔（秒），默认 60
    #[serde(default = "default_heartbeat")]
    pub heartbeat_secs: u32,

    /// 协议版本（驱动出网编码与指令集裁剪），默认 V2022
    #[serde(default)]
    pub version: GbVersion,

    /// 是否启用设备端（默认 false）
    #[serde(default)]
    pub enabled: bool,
}

fn default_ttl() -> u32 {
    3600
}

fn default_heartbeat() -> u32 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_is_disabled_v2022() {
        // 配置段缺失时反序列化为空表，所有字段走 default
        let cfg: GbDevConfig = toml::from_str("").expect("空配置可解析");
        assert!(!cfg.enabled, "默认应禁用");
        assert_eq!(cfg.version, GbVersion::V2022, "默认版本应为 V2022");
        assert_eq!(cfg.register_ttl, 3600);
        assert_eq!(cfg.heartbeat_secs, 60);
    }

    #[test]
    fn config_parse_full() {
        let toml_str = r#"
            platform_uri = "sip:34020000002000000001@192.168.1.1:5060"
            device_id    = "34020000001320000001"
            username     = "34020000001320000001"
            password     = "12345678"
            register_ttl = 7200
            heartbeat_secs = 30
            version      = "v2016"
            enabled      = true
        "#;
        let cfg: GbDevConfig = toml::from_str(toml_str).expect("完整配置可解析");
        assert_eq!(cfg.platform_uri, "sip:34020000002000000001@192.168.1.1:5060");
        assert_eq!(cfg.device_id, "34020000001320000001");
        assert_eq!(cfg.register_ttl, 7200);
        assert_eq!(cfg.heartbeat_secs, 30);
        assert_eq!(cfg.version, GbVersion::V2016);
        assert!(cfg.enabled);
    }
}
