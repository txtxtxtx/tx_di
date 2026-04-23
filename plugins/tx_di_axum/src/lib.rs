mod config;
mod comp;
mod bound;
mod e;

use tx_di_core::BuildContext;
use tx_di_log;
pub use config::*;
pub use comp::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        use tx_di_log;
        // tx_di_log::LogConfig::init_sort();
        // D:\proj\tx_di\configs\di-config.toml
        // C:\a_me\proj\rust\tx_di\configs\di-config.toml
        let mut ctx = BuildContext::new(Some(r"C:\a_me\proj\rust\tx_di\configs\di-config.toml"));
        BuildContext::debug_registry().expect("TODO: panic message");
        ctx.run().await.expect("TODO: panic message")
    }

    #[tokio::test]
    async fn test_ipv4_address_format() {
        let config = WebConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            enable_cors: false,
            max_body_size: 10485760,
            static_dir: "./static".to_string(),
        };
        assert_eq!(config.address(), "127.0.0.1:8080");
        assert!(config.socket_addr().is_ok());
    }

    #[tokio::test]
    async fn test_ipv6_address_format() {
        let config = WebConfig {
            host: "::1".to_string(),
            port: 8080,
            enable_cors: false,
            max_body_size: 10485760,
            static_dir: "./static".to_string(),
        };
        
        // IPv6 地址应该自动添加方括号
        assert_eq!(config.address(), "[::1]:8080");
        assert!(config.socket_addr().is_ok());
    }

    #[tokio::test]
    async fn test_ipv6_wildcard_address() {
        let config = WebConfig {
            host: "::".to_string(),
            port: 3000,
            enable_cors: false,
            max_body_size: 10485760,
            static_dir: "./static".to_string(),
        };
        
        assert_eq!(config.address(), "[::]:3000");
        assert!(config.socket_addr().is_ok());
    }

    #[tokio::test]
    async fn test_ipv6_full_address() {
        let config = WebConfig {
            host: "2001:db8::1".to_string(),
            port: 9090,
            enable_cors: false,
            max_body_size: 10485760,
            static_dir: "./static".to_string(),
        };
        
        assert_eq!(config.address(), "[2001:db8::1]:9090");
        assert!(config.socket_addr().is_ok());
    }
}
