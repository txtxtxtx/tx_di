use serde::Deserialize;
use std::net::SocketAddr;
use tx_di_core::{Component, RIE};
use crate::SipErr;

/// SIP 传输协议类型
#[derive(Debug, Clone, Copy, Deserialize, PartialEq,Default)]
#[serde(rename_all = "lowercase")]
pub enum SipTransport {
    /// 仅启用 UDP（默认）
    #[default]
    Udp,
    /// 仅启用 TCP
    Tcp,
    /// 同时启用 UDP 和 TCP
    Both,
    /// TLS 安全传输（需要配置 `tls` 字段）
    Tls,
    /// WebSocket 传输（用于 WebRTC 等场景）
    Ws,
}

/// SIP 服务器配置
///
/// 通过 TOML 配置文件驱动，与 DI 框架自动集成。
///
/// # 配置文件示例（TOML）
///
/// ## 最简配置（UDP + IPv4，监听 5060）
/// ```toml
/// [sip_config]
/// host = "0.0.0.0"
/// port = 5060
/// ```
///
/// ## IPv6 双栈（UDP + TCP）
/// ```toml
/// [sip_config]
/// host = "::"          # 监听所有 IPv6 接口（自动双栈）
/// port = 5060
/// transport = "both"   # 同时启用 UDP 和 TCP
/// user_agent = "MyApp/1.0"
/// ```
///
/// ## 只监听本地回环（测试用）
/// ```toml
/// [sip_config]
/// host = "127.0.0.1"
/// port = 5060
/// transport = "udp"
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf, init_sort = 10000)]
pub struct SipConfig {
    /// 监听地址
    ///
    /// 支持 IPv4 和 IPv6：
    /// - `"0.0.0.0"` — 监听所有 IPv4 接口
    /// - `"::"` — 监听所有 IPv6 接口（在大多数系统上同时接受 IPv4）
    /// - `"127.0.0.1"` / `"::1"` — 仅本地回环
    ///
    /// 默认值：`"0.0.0.0"`
    #[serde(default = "default_host")]
    pub host: String,

    /// SIP 监听端口，默认 `5060`
    #[serde(default = "default_port")]
    pub port: u16,

    /// 传输层协议，默认 `udp`
    ///
    /// - `udp`：仅 UDP（轻量，推荐开发环境）
    /// - `tcp`：仅 TCP
    /// - `both`：UDP + TCP 双栈
    #[serde(default)]
    pub transport: SipTransport,

    /// User-Agent 字符串，出现在所有 SIP 请求/响应的 User-Agent 头中
    ///
    /// 默认值：`"tx-di-sip/1.0.0"`
    #[serde(default = "default_user_agent")]
    pub user_agent: String,

    /// 对外可见（NAT 穿透）的公网 IP，用于填写 Contact/Via 头
    ///
    /// 当服务部署在 NAT 后面时，填写公网 IP 可保证 SIP 消息路由正确。
    /// 若不填，则使用 `host` 字段的值。
    pub external_ip: Option<String>,

    /// 是否开启详细的 SIP 消息日志，默认 `false`
    #[serde(default)]
    pub log_messages: bool,

    /// SIP 认证域（realm），不填则接受任何域的挑战（Credential.realm = None）
    ///
    /// 部分 SIP 服务器要求客户端在 REGISTER 时指定 realm，
    /// 例如 GB28181 平台通常使用设备 ID 作为 realm。
    #[serde(default)]
    pub realm: Option<String>,

    /// REGISTER/INVITE 等请求的超时重试次数，默认 1
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// SIP 请求超时时间（秒），默认 30
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,

    /// TLS 传输配置（仅 `transport = "tls"` 时使用）
    #[serde(default)]
    pub tls: Option<TlsConfig>,

    /// 是否启用 SIP 服务（默认 true）
    ///
    /// 水平扩容时可设为 `false` 仅做「客户端」（不监听入站），
    /// 或配合其他开关只做「服务端」。
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// 分发队列容量（生产者 → 消费者 channel），默认 10000
    ///
    /// 原通过环境变量 `SIP_QUEUE_SIZE` 配置，现统一进配置文件。
    #[serde(default = "default_dispatch_queue_size")]
    pub dispatch_queue_size: usize,

    /// 并发 handler 上限（Semaphore 背压），默认 1000
    ///
    /// 原通过环境变量 `SIP_MAX_HANDLERS` 配置，现统一进配置文件。
    #[serde(default = "default_max_concurrent_handlers")]
    pub max_concurrent_handlers: usize,
}

/// TLS 传输配置
#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    /// 证书 PEM 文件路径
    pub cert_pem: String,
    /// 私钥 PEM 文件路径
    pub key_pem: String,
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            transport: SipTransport::default(),
            user_agent: default_user_agent(),
            external_ip: None,
            log_messages: false,
            realm: None,
            retry_count: default_retry_count(),
            request_timeout_secs: default_request_timeout_secs(),
            tls: None,
            enabled: default_enabled(),
            dispatch_queue_size: default_dispatch_queue_size(),
            max_concurrent_handlers: default_max_concurrent_handlers(),
        }
    }
}

impl SipConfig {
    /// 获取完整的绑定地址（`host:port`）
    pub fn bind_addr(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// 将配置转换为 `SocketAddr`
    pub fn socket_addr(&self) -> RIE<SocketAddr> {
        let raw = format!("{}:{}", self.host, self.port);
        Ok(raw.parse::<SocketAddr>()
            .map_err(|_| SipErr::InvalidAddress)?)
    }

    /// 获取对外可见的 IP（用于 Contact/Via 头）
    pub fn contact_ip(&self) -> &str {
        self.external_ip.as_deref().unwrap_or(&self.host)
    }

    /// 是否启用 UDP
    pub fn enable_udp(&self) -> bool {
        matches!(self.transport, SipTransport::Udp | SipTransport::Both)
    }

    /// 是否启用 TCP
    pub fn enable_tcp(&self) -> bool {
        matches!(self.transport, SipTransport::Tcp | SipTransport::Both)
    }
}

pub(crate) fn default_host() -> String {
    "0.0.0.0".to_string()
}

pub(crate) fn default_port() -> u16 {
    5060
}

pub(crate) fn default_user_agent() -> String {
    "tx-di-sip/1.0.0".to_string()
}

pub(crate) fn default_retry_count() -> u32 {
    1
}

pub(crate) fn default_request_timeout_secs() -> u64 {
    30
}

pub(crate) fn default_enabled() -> bool {
    true
}

pub(crate) fn default_dispatch_queue_size() -> usize {
    10_000
}

pub(crate) fn default_max_concurrent_handlers() -> usize {
    1000
}
