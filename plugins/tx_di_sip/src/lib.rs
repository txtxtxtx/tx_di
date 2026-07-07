//! # tx_di_sip
//!
//! 基于 [rsipstack](https://crates.io/crates/rsipstack) 的 SIP 服务插件，
//! 集成到 tx-di 依赖注入框架中，提供开箱即用的 SIP 协议能力。
//!
//! ## 功能概览
//!
//! - **IPv4 / IPv6 双栈支持** — 通过配置 `host` 字段选择监听地址
//! - **UDP + TCP 双传输层** — 可单独或同时启用
//! - **消息处理注册** — 类似 axum 路由的 [`SipRouter::add_handler`] 机制
//! - **中间件洋葱链** — 作用在 [`SipTx`] 上（认证/日志/NAT 修正/限流）
//! - **消息发送接口** — [`SipSender`] 提供 `register()`/`invite()`/`send_message()` 等
//! - **优雅停止** — 通过 `CancellationToken` 支持 shutdown
//!
//! ## 快速开始
//!
//! ### 1. 添加依赖
//!
//! 在 `Cargo.toml` 中：
//! ```toml
//! [dependencies]
//! tx_di_sip = { path = "plugins/tx_di_sip" }
//! ```
//!
//! ### 2. 配置文件（`di-config.toml`）
//!
//! ```toml
//! [sip_config]
//! host     = "0.0.0.0"    # 监听地址（IPv4）
//! port     = 5060          # SIP 端口
//! transport = "udp"        # 传输协议: udp / tcp / both
//! user_agent = "MyApp/1.0"
//! enabled = true           # 是否启用 SIP 服务
//! ```
//!
//! ### 3. 注册消息处理器 & 启动
//!
//! ```rust,ignore
//! use tx_di_sip::{SipPlugin, SipRouter, SipTx};
//! use rsipstack::sip::StatusCode;
//! use tx_di_core::BuildContext;
//!
//! // 启动前注册处理器
//! let router = SipRouter::new();
//! router.add_handler(Some("REGISTER"), 0, |tx: SipTx| async move {
//!     println!("收到 REGISTER: {}", tx.request().method);
//!     tx.reply(StatusCode::OK).await?;
//!     Ok(())
//! });
//!
//! // 启动 DI 框架
//! let mut ctx = BuildContext::new(Some("configs/di-config.toml"));
//! ctx.build_and_run().await.unwrap();
//! ```

mod config;
mod comp;
pub mod err;
mod handler;
pub mod auth;
mod middleware;
mod sender;
pub mod client;
mod sip_tx;
mod dialog;

pub use config::*;
pub use comp::*;
pub use err::SipErr;
pub use handler::SipRouter;
pub use middleware::{SipMiddleware, SipNextFn, SipNextFut};
pub use sender::SipSender;
pub use client::{SipClient, SipClientConfig};
pub use sip_tx::SipTx;
pub use dialog::{DialogKey, InDialogTable};

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};
    use tx_di_core::RIE;

    // ─────────────────────────────────────────────────────────────────────
    //  SipConfig 单元测试
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn config_default_host_is_ipv4_any() {
        let cfg = default_config();
        assert_eq!(cfg.host, "0.0.0.0");
    }

    #[test]
    fn config_default_port_5060() {
        let cfg = default_config();
        assert_eq!(cfg.port, 5060);
    }

    #[test]
    fn config_default_transport_both() {
        let cfg = default_config();
        assert_eq!(cfg.transport, SipTransport::Both);
    }

    #[test]
    fn config_default_user_agent() {
        let cfg = default_config();
        assert_eq!(cfg.user_agent, "tx-di-sip/1.0.0");
    }

    #[test]
    fn config_default_external_ip_none() {
        let cfg = default_config();
        assert!(cfg.external_ip.is_none());
    }

    #[test]
    fn config_default_log_messages_false() {
        let cfg = default_config();
        assert!(!cfg.log_messages);
    }

    #[test]
    fn config_default_enabled_true() {
        let cfg = default_config();
        assert!(cfg.enabled);
    }

    #[test]
    fn config_default_perf() {
        let cfg = default_config();
        assert_eq!(cfg.dispatch_queue_size, 10_000);
        assert_eq!(cfg.max_concurrent_handlers, 1000);
    }

    // ── enable_udp / enable_tcp ──────────────────────────────────────

    #[test]
    fn transport_udp_enables_only_udp() {
        let cfg = SipConfig {
            transport: SipTransport::Udp,
            ..default_config()
        };
        assert!(cfg.enable_udp(), "UDP 模式应启用 UDP");
        assert!(!cfg.enable_tcp(), "UDP 模式不应启用 TCP");
    }

    #[test]
    fn transport_tcp_enables_only_tcp() {
        let cfg = SipConfig {
            transport: SipTransport::Tcp,
            ..default_config()
        };
        assert!(!cfg.enable_udp(), "TCP 模式不应启用 UDP");
        assert!(cfg.enable_tcp(), "TCP 模式应启用 TCP");
    }

    #[test]
    fn transport_both_enables_all() {
        let cfg = SipConfig {
            transport: SipTransport::Both,
            ..default_config()
        };
        assert!(cfg.enable_udp(), "Both 模式应启用 UDP");
        assert!(cfg.enable_tcp(), "Both 模式应启用 TCP");
    }

    // ── socket_addr ──────────────────────────────────────────────────

    #[test]
    fn socket_addr_valid_ipv4() {
        let cfg = SipConfig {
            host: "192.168.1.100".to_string(),
            port: 5061,
            ..default_config()
        };
        let addr: SocketAddr = cfg.socket_addr().expect("解析 IPv4 地址成功");
        assert_eq!(addr.ip(), Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(addr.port(), 5061);
    }

    #[test]
    fn socket_addr_loopback() {
        let cfg = SipConfig {
            host: "127.0.0.1".to_string(),
            ..default_config()
        };
        let addr: SocketAddr = cfg.socket_addr().unwrap();
        assert_eq!(addr.ip(), Ipv4Addr::LOCALHOST);
        assert_eq!(addr.port(), 5060);
    }

    #[test]
    fn socket_addr_invalid_returns_error() {
        let cfg = SipConfig {
            host: "not-a-valid-ip".to_string(),
            ..default_config()
        };
        let result = cfg.socket_addr();
        assert!(result.is_err(), "无效 IP 应返回 Err");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("无效的 SIP 地址"),
            "错误信息应包含描述: {}",
            err_msg
        );
    }

    // ── bind_addr（IPv6 方括号处理）────────────────────────────────

    #[test]
    fn bind_addr_ipv4_no_brackets() {
        let cfg = SipConfig {
            host: "0.0.0.0".to_string(),
            port: 5060,
            ..default_config()
        };
        assert_eq!(cfg.bind_addr(), "0.0.0.0:5060");
    }

    #[test]
    fn bind_addr_ipv6_with_brackets() {
        let cfg = SipConfig {
            host: "::".to_string(),
            port: 5060,
            ..default_config()
        };
        assert_eq!(cfg.bind_addr(), "[::]:5060");
    }

    #[test]
    fn bind_addr_ipv6_already_bracketed() {
        let cfg = SipConfig {
            host: "::1".to_string(),
            port: 5070,
            ..default_config()
        };
        assert_eq!(cfg.bind_addr(), "[::1]:5070");
    }

    #[test]
    fn bind_addr_custom_port() {
        let cfg = SipConfig {
            host: "10.0.0.5".to_string(),
            port: 5080,
            ..default_config()
        };
        assert_eq!(cfg.bind_addr(), "10.0.0.5:5080");
    }

    // ── contact_ip ───────────────────────────────────────────────────

    #[test]
    fn contact_ip_falls_back_to_host_when_external_empty() {
        let cfg = SipConfig {
            host: "192.168.1.1".to_string(),
            external_ip: None,
            ..default_config()
        };
        assert_eq!(cfg.contact_ip(), "192.168.1.1");
    }

    #[test]
    fn contact_ip_uses_external_when_set() {
        let cfg = SipConfig {
            host: "192.168.1.1".to_string(),
            external_ip: Some("203.0.113.50".to_string()),
            ..default_config()
        };
        assert_eq!(cfg.contact_ip(), "203.0.113.50");
    }

    // ─────────────────────────────────────────────────────────────────────
    //  SipTransport 反序列化测试
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn transport_via_config_udp() {
        let cfg: SipConfig = toml::from_str(r#"
            host = "0.0.0.0"
            port = 5060
            transport = "udp"
        "#).unwrap();
        assert_eq!(cfg.transport, SipTransport::Udp);
    }

    #[test]
    fn transport_via_config_tcp() {
        let cfg: SipConfig = toml::from_str(r#"
            host = "0.0.0.0"
            port = 5060
            transport = "tcp"
        "#).unwrap();
        assert_eq!(cfg.transport, SipTransport::Tcp);
    }

    #[test]
    fn transport_via_config_both() {
        let cfg: SipConfig = toml::from_str(r#"
            host = "0.0.0.0"
            port = 5060
            transport = "both"
        "#).unwrap();
        assert_eq!(cfg.transport, SipTransport::Both);
    }

    #[test]
    fn transport_default_is_both() {
        let cfg: SipConfig = toml::from_str("host = \"0.0.0.0\"\nport = 5060").unwrap();
        assert_eq!(cfg.transport, SipTransport::Both);
    }

    // ─────────────────────────────────────────────────────────────────────
    //  SipConfig TOML 完整反序列化测试
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn config_from_toml_minimal() {
        let toml_str = r#"
            host = "0.0.0.0"
            port = 5060
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).expect("最小配置解析成功");

        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 5060);
        assert_eq!(cfg.transport, SipTransport::Both);    // 默认值
        assert_eq!(cfg.user_agent, "tx-di-sip/1.0.0");   // 默认值
        assert!(cfg.external_ip.is_none());               // 默认值
        assert!(!cfg.log_messages);                       // 默认值
        assert!(cfg.enabled);                             // 默认值
        assert_eq!(cfg.dispatch_queue_size, 10_000);      // 默认值
        assert_eq!(cfg.max_concurrent_handlers, 1000);    // 默认值
    }

    #[test]
    fn config_from_toml_full() {
        let toml_str = r#"
            host = "::"
            port = 5062
            transport = "both"
            user_agent = "GB28101-Srv/2.0"
            external_ip = "203.0.113.10"
            log_messages = true
            enabled = false
            dispatch_queue_size = 20000
            max_concurrent_handlers = 2000
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).expect("完整配置解析成功");

        assert_eq!(cfg.host, "::");
        assert_eq!(cfg.port, 5062);
        assert_eq!(cfg.transport, SipTransport::Both);
        assert_eq!(cfg.user_agent, "GB28101-Srv/2.0");
        assert_eq!(cfg.external_ip.as_deref().unwrap(), "203.0.113.10");
        assert!(cfg.log_messages);
        assert!(!cfg.enabled);
        assert_eq!(cfg.dispatch_queue_size, 20000);
        assert_eq!(cfg.max_concurrent_handlers, 2000);
    }

    #[test]
    fn config_from_toml_tcp_only() {
        let toml_str = r#"
            host      = "127.0.0.1"
          port      = 15060
            transport = "tcp"
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 15060);
        assert_eq!(cfg.transport, SipTransport::Tcp);
        assert!(!cfg.enable_udp());
        assert!(cfg.enable_tcp());
    }

    #[test]
    fn config_from_toml_ipv4_both_transports() {
        let toml_str = r#"
            host       = "0.0.0.0"
            port       = 5060
            transport  = "both"
            user_agent = "tx-di-sip/0.1.0"
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).unwrap();

        assert!(cfg.enable_udp());
        assert!(cfg.enable_tcp());
        assert_eq!(cfg.socket_addr().unwrap().port(), 5060);
    }

    // ── TlsConfig 双向 TLS 反序列化 ──────────────────────────────────────

    #[test]
    fn tls_config_parse_basic() {
        let cfg: SipConfig = toml::from_str(
            r#"
            host = "0.0.0.0"
            port = 5061
            transport = "tls"
            [tls]
            cert_pem = "server.pem"
            key_pem  = "server.key"
        "#,
        )
        .unwrap();
        let tls = cfg.tls.expect("应解析出 tls 配置");
        assert_eq!(tls.cert_pem, "server.pem");
        assert_eq!(tls.key_pem, "server.key");
        // 双向 TLS 字段默认为 None
        assert!(tls.ca_certs.is_none());
        assert!(tls.client_cert.is_none());
        assert!(tls.client_key.is_none());
    }

    #[test]
    fn tls_config_parse_mutual_fields() {
        let cfg: SipConfig = toml::from_str(
            r#"
            host = "0.0.0.0"
            port = 5061
            transport = "tls"
            [tls]
            cert_pem    = "server.pem"
            key_pem     = "server.key"
            ca_certs    = "ca.pem"
            client_cert = "client.pem"
            client_key  = "client.key"
        "#,
        )
        .unwrap();
        let tls = cfg.tls.expect("应解析出 tls 配置");
        assert_eq!(tls.ca_certs.as_deref(), Some("ca.pem"));
        assert_eq!(tls.client_cert.as_deref(), Some("client.pem"));
        assert_eq!(tls.client_key.as_deref(), Some("client.key"));
    }

    // ─────────────────────────────────────────────────────────────────────
    //  SipRouter 测试（注册 / 分发 / 清空 / 计数）
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn router_lifecycle() {
        let sip_router = SipRouter::new();

        // ── 逐个注册并计数 ─────────────────────────────────────────
        sip_router.add_handler(Some("REGISTER"), 0, dummy_handler);
        assert_eq!(sip_router.handler_count(), 1);

        sip_router.add_handler(Some("INVITE"), 0, dummy_handler);
        sip_router.add_handler(Some("OPTIONS"), 10, dummy_handler);
        assert_eq!(sip_router.handler_count(), 3);

        // ── clear 重置 ──────────────────────────────────────────────
        sip_router.add_handler(Some("BYE"), 0, dummy_handler);
        sip_router.add_handler(None::<&str>, 99, dummy_handler); // catch-all
        assert_eq!(sip_router.handler_count(), 5);

        sip_router.clear();
        assert_eq!(sip_router.handler_count(), 0);

        // ── 方法名自动转大写 ────────────────────────────────────────
        sip_router.add_handler(Some("register"), 0, dummy_handler);  // 小写 → 大写
        sip_router.add_handler(Some("Invite"), 0, dummy_handler);   // 混合 → 大写
        sip_router.add_handler(Some("options"), 0, dummy_handler);  // 小写 → 大写
        assert_eq!(sip_router.handler_count(), 3);

        // ── catch-all (method = None) ──────────────────────────────
        sip_router.clear();
        sip_router.add_handler(None::<&str>, 100, dummy_handler);
        assert_eq!(sip_router.handler_count(), 1);

        // ── 同方法多优先级 ─────────────────────────────────────────
        sip_router.clear();
        sip_router.add_handler(Some("REGISTER"), 0, |_tx| async move { Ok(()) });
        sip_router.add_handler(Some("REGISTER"), 10, |_tx| async move { Ok(()) });
        sip_router.add_handler(Some("REGISTER"), 5, |_tx| async move { Ok(()) });
        assert_eq!(sip_router.handler_count(), 3);

        // 最终清理
        sip_router.clear();
    }

    // ─────────────────────────────────────────────────────────────────────
    //  边界情况与属性测试
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn config_extreme_port_values() {
        let cfg_min = SipConfig {
            port: 1,
            ..default_config()
        };
        assert_eq!(cfg_min.socket_addr().unwrap().port(), 1);

        let cfg_max = SipConfig {
            port: 65535,
            ..default_config()
        };
        assert_eq!(cfg_max.socket_addr().unwrap().port(), 65535);
    }

    #[test]
    fn bind_addr_hostname_style_no_colon() {
        let cfg = SipConfig {
            host: "sip-server.local".to_string(),
            port: 5060,
            ..default_config()
        };
        assert_eq!(cfg.bind_addr(), "sip-server.local:5060");
    }

    #[test]
    fn sip_transport_partial_eq() {
        assert_eq!(SipTransport::Udp, SipTransport::Udp);
        assert_ne!(SipTransport::Udp, SipTransport::Tcp);
        assert_ne!(SipTransport::Tcp, SipTransport::Both);
    }

    #[test]
    fn sip_transport_debug_format() {
        let debug_str = format!("{:?}", SipTransport::Both);
        assert!(debug_str.contains("Both"));
    }

    // ─────────────────────────────────────────────────────────────────────
    //  SipTx 测试（fake 模式）
    // ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn sip_tx_fake_reply_is_idempotent() {
        use rsipstack::sip::StatusCode;
        let req = rsipstack::sip::Request {
            method: rsipstack::sip::Method::Register,
            uri: rsipstack::sip::Uri::try_from("sip:registrar@example.com").unwrap(),
            headers: vec![].into(),
            body: vec![],
            version: rsipstack::sip::Version::V2,
        };
        let (tx, recorder) = SipTx::fake(req);

        tx.reply(StatusCode::OK).await.unwrap();
        // 第二次回复应被幂等忽略
        tx.reply(StatusCode::Forbidden).await.unwrap();

        assert!(tx.replied());
        assert_eq!(*recorder.lock().await, Some(StatusCode::OK));
    }

    // ─────────────────────────────────────────────────────────────────────
    //  辅助函数
    // ─────────────────────────────────────────────────────────────────────

    /// 创建一个默认 SipConfig 用于测试
    fn default_config() -> SipConfig {
        SipConfig {
            host: default_host(),
            port: default_port(),
            transport: SipTransport::Both,
            user_agent: default_user_agent(),
            external_ip: None,
            log_messages: false,
            realm: None,
            retry_count: 0,
            request_timeout_secs: 0,
            tls: None,
            enabled: true,
            dispatch_queue_size: 10_000,
            max_concurrent_handlers: 1000,
        }
    }

    /// 空操作 handler：接收 SipTx 并直接返回 Ok
    async fn dummy_handler(_tx: SipTx) -> RIE<()> {
        Ok(())
    }
}
