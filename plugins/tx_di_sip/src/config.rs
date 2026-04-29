use serde::Deserialize;
use std::net::SocketAddr;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};

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
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]
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
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            transport: SipTransport::default(),
            user_agent: default_user_agent(),
            external_ip: Some(default_host()),
            log_messages: false,
        }
    }
}

impl CompInit for SipConfig {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        Ok(())
    }
    fn init_sort() -> i32 {
        i32::MAX - 2
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
        raw.parse::<SocketAddr>()
            .map_err(|e| tx_di_core::IE::Other(format!("无效的 SIP 地址 '{}': {}", raw, e)))
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
