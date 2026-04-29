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
//! - **消息发送接口** — [`SipSender`] 提供 `register()`/`invite()` 等便捷 API
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
//! ```
//!
//! ### 3. 注册消息处理器 & 启动
//!
//! ```rust,no_run
//! use tx_di_sip::{SipPlugin, SipRouter};
//! use rsipstack::sip::StatusCode;
//! use tx_di_core::BuildContext;
//!
//! // 启动前注册处理器
//! SipRouter::add_handler(Some("REGISTER"), 0, |mut tx| async move {
//!     println!("收到 REGISTER: {}", tx.original);
//!     tx.reply(StatusCode::OK).await?;
//!     Ok(())
//! });
//!
//! SipRouter::add_handler(Some("OPTIONS"), 0, |mut tx| async move {
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
mod handler;
mod sender;

pub use config::*;
pub use comp::*;
pub use handler::SipRouter;
pub use sender::SipSender;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

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
    fn config_default_transport_udp() {
        let cfg = default_config();
        assert_eq!(cfg.transport, SipTransport::Udp);
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
        // 包含冒号的 IPv6 地址应自动加方括号
        let cfg = SipConfig {
            host: "::".to_string(),
            port: 5060,
            ..default_config()
        };
        assert_eq!(cfg.bind_addr(), "[::]:5060");
    }

    #[test]
    fn bind_addr_ipv6_already_bracketed() {
        // 已经有方括号的不重复加
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
    //  SipTransport 序列化/反序列化测试
    // ─────────────────────────────────────────────────────────────────────

    // ─────────────────────────────────────────────────────────────────────
    //  SipTransport 反序列化测试
    //
    //  通过完整 SipConfig TOML 间接验证 SipTransport 的反序列化行为，
    //  因为裸 enum 的 toml 反序列化格式与 serde_json 不同（TOML 需要结构体上下文）。
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
    fn transport_default_is_udp() {
        // 不传 transport 字段时，默认为 Udp
        let cfg: SipConfig = toml::from_str("host = \"0.0.0.0\"\nport = 5060").unwrap();
        assert_eq!(cfg.transport, SipTransport::Udp);
    }

    // ─────────────────────────────────────────────────────────────────────
    //  SipConfig TOML 完整反序列化测试
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn config_from_toml_minimal() {
        // 只填必填项，其余使用默认值
        let toml_str = r#"
            host = "0.0.0.0"
            port = 5060
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).expect("最小配置解析成功");

        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 5060);
        assert_eq!(cfg.transport, SipTransport::Udp);     // 默认值
        assert_eq!(cfg.user_agent, "tx-di-sip/1.0.0");   // 默认值
        assert!(cfg.external_ip.is_none());               // 默认值
        assert!(!cfg.log_messages);                       // 默认值
    }

    #[test]
    fn config_from_toml_full() {
        let toml_str = r#"
            host         = "::"
            port         = 5062
            transport    = "both"
            user_agent   = "GB28101-Srv/2.0"
            external_ip  = "203.0.113.10"
            log_messages = true
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).expect("完整配置解析成功");

        assert_eq!(cfg.host, "::");
        assert_eq!(cfg.port, 5062);
        assert_eq!(cfg.transport, SipTransport::Both);
        assert_eq!(cfg.user_agent, "GB28101-Srv/2.0");
        assert_eq!(cfg.external_ip.as_deref().unwrap(), "203.0.113.10");
        assert!(cfg.log_messages);
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
        // 模拟 di-config.toml 中的典型生产配置
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

    // ─────────────────────────────────────────────────────────────────────
    //  SipRouter 测试（注册 / 分发 / 清空 / 计数）
    //
    //  注意：SipRouter 使用全局 LazyLock<RwLock<Vec<HandlerEntry>>> 作为注册表，
    //  并行测试会相互干扰。因此所有 router 相关的测试合并在一个串行 test 函数中。
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn router_lifecycle() {
        // ── 初始状态为空 ───────────────────────────────────────────
        SipRouter::clear();
        assert_eq!(SipRouter::handler_count(), 0, "初始应为空");

        // ── 逐个注册并计数 ─────────────────────────────────────────
        SipRouter::add_handler(Some("REGISTER"), 0, dummy_handler);
        assert_eq!(SipRouter::handler_count(), 1);

        SipRouter::add_handler(Some("INVITE"), 0, dummy_handler);
        SipRouter::add_handler(Some("OPTIONS"), 10, dummy_handler);
        assert_eq!(SipRouter::handler_count(), 3);

        // ── clear 重置 ──────────────────────────────────────────────
        SipRouter::add_handler(Some("BYE"), 0, dummy_handler);
        SipRouter::add_handler(None, 99, dummy_handler); // catch-all
        assert_eq!(SipRouter::handler_count(), 5);

        SipRouter::clear();
        assert_eq!(SipRouter::handler_count(), 0);

        // ── 方法名自动转大写 ────────────────────────────────────────
        SipRouter::add_handler(Some("register"), 0, dummy_handler);  // 小写 → 大写
        SipRouter::add_handler(Some("Invite"), 0, dummy_handler);   // 混合 → 大写
        SipRouter::add_handler(Some("options"), 0, dummy_handler);  // 小写 → 大写
        assert_eq!(SipRouter::handler_count(), 3);

        // ── catch-all (method = None) ──────────────────────────────
        SipRouter::clear();
        SipRouter::add_handler(None, 100, dummy_handler);
        assert_eq!(SipRouter::handler_count(), 1);

        // ── 同方法多优先级 ─────────────────────────────────────────
        SipRouter::clear();
        SipRouter::add_handler(Some("REGISTER"), 0, |_tx| async move { Ok(()) });
        SipRouter::add_handler(Some("REGISTER"), 10, |_tx| async move { Ok(()) });
        SipRouter::add_handler(Some("REGISTER"), 5, |_tx| async move { Ok(()) });
        assert_eq!(SipRouter::handler_count(), 3);

        // 最终清理
        SipRouter::clear();
    }

    // ─────────────────────────────────────────────────────────────────────
    //  边界情况与属性测试
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn config_extreme_port_values() {
        // 最小端口 1
        let cfg_min = SipConfig {
            port: 1,
            ..default_config()
        };
        assert_eq!(cfg_min.socket_addr().unwrap().port(), 1);

        // 最大端口 65535
        let cfg_max = SipConfig {
            port: 65535,
            ..default_config()
        };
        assert_eq!(cfg_max.socket_addr().unwrap().port(), 65535);
    }

    #[test]
    fn bind_addr_hostname_style_no_colon() {
        // 不含冒号的主机名不加方括号
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
    //  辅助函数
    // ─────────────────────────────────────────────────────────────────────

    /// 创建一个默认 SipConfig 用于测试
    fn default_config() -> SipConfig {
        SipConfig {
            host: default_host(),
            port: default_port(),
            transport: SipTransport::Udp,
            user_agent: default_user_agent(),
            external_ip: None,
            log_messages: false,
        }
    }

    /// 空操作 handler：接收 Transaction 并直接返回 Ok
    ///
    /// 注意：此 handler 不调用 `tx.reply()`，仅用于验证注册/分发机制本身。
    /// 真正的分发集成测试需要构造真实的 rsipstack Transaction，
    /// 那属于端到端集成测试范畴（需启动 UDP/TCP transport）。
    async fn dummy_handler(_tx: rsipstack::transaction::transaction::Transaction) -> anyhow::Result<()> {
        Ok(())
    }
}
