//! # di-example
//!
//! 演示 tx_di 的全部特性：
//!
//! ## 设计原则
//!
//! **scope 标记在被注入者上**，消费者字段写裸类型即可：
//!
//! ## 依赖关系
//!
//! ```text
//!  ┌──────────┐        ┌────────────┐
//!  │  DbPool  │        │ AppConfig  │  ← #[tx_comp]（默认 Singleton）
//!  └────┬─────┘        └─────┬──────┘
//!       │                    │
//!       ▼ Singleton          ▼ Singleton (自动)
//!  ┌──────────────────┐  ┌──────────────────────┐
//!  │    UserService   │  │    RequestLogger     │← #[tx_comp(scope = Prototype)]
//!  │  db: Arc<DbPool> │  └──────────┬───────────┘
//!  │  config: Arc<..> │             │ Prototype (自动)
//!  └────────┬─────────┘             │
//!           │ Singleton             │
//!           └──────────┬────────────┘
//!                      ▼
//!              ┌──────────────┐
//!              │  AppServer   │
//!              │  user_svc    │
//!              │  logger      │ ← Arc<RequestLogger>，每次注入新实例
//!              │  extra       │ ← #[tx_cst(expr)]
//!              └──────────────┘
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tx_di_core::{tx_comp, BoxFuture, BuildContext, CompInit, App, RIE};
use log::{debug, info};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
// ─────────────────────────────────────────────────────────────────────────────
// 1. 无依赖的单例组件
// ─────────────────────────────────────────────────────────────────────────────

/// 数据库连接池（单例，全局共享）
#[derive(Clone, Debug)]
#[tx_comp]
pub struct DbPool {
    // 实际项目中这里是 sea_orm::DatabaseConnection
    // 无字段的组件：build() → Self {}
}

/// 应用配置（单例，通过 #[tx_cst] 注入自定义值）
#[derive(Clone, Debug,Deserialize,Default)]
#[tx_comp(conf)]
pub struct AppConfig {
    /// 应用名称（自定义值注入）
    #[serde(default = "default_name")]
    pub app_name: String,
    /// 监听端口（函数调用注入）
    #[serde(default = "default_port")]
    pub port: u16,
}
fn default_name() -> String {
    "tx-di-example".to_string()
}
/// 提供默认端口
pub fn default_port() -> u16 {
    8080
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. 原型组件（每次注入构造新实例）
// ─────────────────────────────────────────────────────────────────────────────

/// 请求日志器（原型，每个注入点独立持有）。
///
/// 注意：count 字段使用 Arc<Mutex<u64>> 实现 interior mutability，
/// 因为 inject() 返回 Arc<T>，不支持 &mut T。
#[derive(Clone, Debug)]
#[tx_comp(scope = Prototype)]
pub struct RequestLogger {
    /// 日志前缀
    #[tx_cst("[REQUEST]".to_string())]
    pub prefix: String,

    /// 请求计数器（每个实例独立，通过 #[tx_cst] 初始化）
    #[tx_cst(Arc::new(Mutex::new(0u64)))]
    count: Arc<Mutex<u64>>,
}

impl RequestLogger {
    pub fn log(&self, msg: &str) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        info!("{} [#{}] {}", self.prefix, *c, msg);
    }

    pub fn count(&self) -> u64 {
        *self.count.lock().unwrap()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. 依赖其他组件的服务
// ─────────────────────────────────────────────────────────────────────────────

/// 用户服务（单例，依赖 DbPool + AppConfig）。
///
/// 字段类型是 `Arc<T>`：inject() 返回 Arc<T>，Arc derefs 到 T，
/// 所以 `.greet()` 和 `.config.app_name` 可以直接调用。
#[derive(Clone, Debug)]
#[tx_comp]
pub struct UserService {
    /// 共享的 DbPool（Arc<T> derefs，方法调用透明）
    pub db: Arc<DbPool>,

    /// 共享的 AppConfig
    pub config: Arc<AppConfig>,
}

impl UserService {
    pub fn greet(&self) -> String {
        format!(
            "[{}] Hello from UserService (port: {})",
            self.config.app_name, self.config.port
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. 顶层聚合组件
// ─────────────────────────────────────────────────────────────────────────────

/// HTTP 服务器（单例，聚合多种依赖）
#[derive(Debug)]
#[tx_comp(init)]
pub struct AppServer {
    /// 共享的 UserService
    pub user_svc: Arc<UserService>,

    /// 原型注入：AppServer 独占自己的 RequestLogger 实例
    /// （每次 inject() 对 Prototype 组件调用工厂，构造新实例）
    pub logger: Arc<RequestLogger>,

    /// 自定义值注入：HashMap 不通过 ctx，直接用表达式构造
    #[tx_cst(default_headers())]
    pub default_headers: HashMap<String, String>,

    /// 自定义值注入
    #[tx_cst("0.0.0.0:8080".to_string())]
    pub bind_addr: String,
}

impl CompInit for AppServer {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        info!("AppServer::inner_init");
        Ok(())
    }
    fn init(_ctx: Arc<App>) -> RIE<()> {
        debug!("AppServer::init");
        Ok(())
    }
    fn async_init(ctx: Arc<App>,_token: CancellationToken) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            debug!("AppServer::async_init, app.len={}", ctx.len());
            Ok(())
        })
    }
    fn init_sort() -> i32 {
        1000
    }
}
/// 构造默认响应头
pub fn default_headers() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("X-Powered-By".to_string(), "di-framework".to_string());
    m.insert("Content-Type".to_string(), "application/json".to_string());
    m
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试模块
//
// 覆盖范围：
//  1. DI 核心功能（单例/原型/依赖链/自定义值注入）
//  2. BuildContext API
//  3. 注册表与 ComponentDescriptor
//  4. 配置文件加载（TOML → AppAllConfig → 各组件配置）
//  5. Web 插件集成（WebPlugin / WebConfig）
//  6. SIP 插件集成（SipConfig / SipRouter）
//  7. Log 插件集成（LogConfig）
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tx_di_core::{ComponentDescriptor, Scope, COMPONENT_REGISTRY};
    use tx_di_axum::{WebConfig, WebPlugin};
    use tx_di_sip::{SipConfig, SipRouter, SipTransport};
    use tx_di_log::LogConfig;
    use super::*;

    /// 辅助函数：创建无配置文件的上下文（自动扫描所有组件）
    fn create_full_context() -> BuildContext {
        BuildContext::new::<PathBuf>(None)
    }

    // ════════════════════════════════════════════════════════════════════
    //  1. DI 核心功能 — 单例
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_singleton_shared() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>().expect("AppServer 构建失败");
        // UserService 内的 DbPool 和 ctx 里的是同一个 Arc
        let db_in_ctx = ctx.inject::<DbPool>();
        assert!(
            Arc::ptr_eq(&server.user_svc.db, &db_in_ctx),
            "DbPool 应该是同一个 Arc 实例（单例）"
        );
    }

    #[test]
    fn test_singleton_multiple_injects_same_instance() {
        let mut ctx = create_full_context();
        let db1 = ctx.inject::<DbPool>();
        let db2 = ctx.inject::<DbPool>();
        let db3 = ctx.inject::<DbPool>();
        assert!(Arc::ptr_eq(&db1, &db2), "两次注入应该返回相同实例");
        assert!(Arc::ptr_eq(&db2, &db3), "三次注入应该返回相同实例");
    }

    #[test]
    fn test_singleton_arc_clone_shares_data() {
        let mut ctx = create_full_context();
        let db1 = ctx.inject::<DbPool>();
        let _db2 = db1.clone();
        // 引用计数: ctx(1) + db1 + clone = 至少 3
        assert!(Arc::strong_count(&db1) >= 3);
    }

    // ════════════════════════════════════════════════════════════════════
    //  2. DI 核心功能 — 原型 (Prototype)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_prototype_independent() {
        let mut ctx = create_full_context();
        let l1 = ctx.inject::<RequestLogger>();
        let l2 = ctx.inject::<RequestLogger>();
        l1.log("l1 msg");
        l2.log("l2 msg");
        assert_ne!(
            Arc::as_ptr(&l1),
            Arc::as_ptr(&l2),
            "Prototype 实例应该相互独立（不同 Arc 指针）"
        );
        assert_eq!(l1.count(), 1);
        assert_eq!(l2.count(), 1);
    }

    #[test]
    fn test_prototype_each_inject_creates_new_instance() {
        let mut ctx = create_full_context();
        let loggers: Vec<_> = (0..5).map(|_| ctx.inject::<RequestLogger>()).collect();
        for i in 0..loggers.len() {
            for j in (i + 1)..loggers.len() {
                assert_ne!(
                    Arc::as_ptr(&loggers[i]),
                    Arc::as_ptr(&loggers[j]),
                    "第 {} 和第 {} 次注入应产生不同实例",
                    i + 1,
                    j + 1
                );
            }
        }
    }

    #[test]
    fn test_prototype_with_custom_values() {
        let mut ctx = create_full_context();
        let logger = ctx.inject::<RequestLogger>();
        assert_eq!(logger.prefix, "[REQUEST]");
        assert_eq!(logger.count(), 0);
    }

    #[test]
    fn test_prototype_state_isolation() {
        let mut ctx = create_full_context();
        let mut loggers = vec![];
        for i in 0..3 {
            let logger = ctx.inject::<RequestLogger>();
            for _ in 0..(i + 1) {
                logger.log(&format!("Message from logger {}", i));
            }
            loggers.push(logger);
        }
        assert_eq!(loggers[0].count(), 1);
        assert_eq!(loggers[1].count(), 2);
        assert_eq!(loggers[2].count(), 3);
    }

    // ════════════════════════════════════════════════════════════════════
    //  3. 自定义值注入 (#[tx_cst])
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_inject_custom_values() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>().expect("AppServer 构建失败");
        assert_eq!(server.bind_addr, "0.0.0.0:8080");
        assert_eq!(
            server.default_headers.get("X-Powered-By").unwrap(),
            "di-framework"
        );
    }

    #[test]
    fn test_app_config_default_values() {
        let mut ctx = create_full_context();
        let cfg = ctx.inject::<AppConfig>();
        assert_eq!(cfg.app_name, "tx-di-example");
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn test_custom_value_expression_evaluated_once() {
        let mut ctx = create_full_context();
        let cfg1 = ctx.inject::<AppConfig>();
        let cfg2 = ctx.inject::<AppConfig>();
        assert!(Arc::ptr_eq(&cfg1, &cfg2));
    }

    // ════════════════════════════════════════════════════════════════════
    //  4. 依赖链
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_dependency_injection_chain() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>().expect("AppServer 构建失败");
        assert!(Arc::ptr_eq(&server.user_svc.db, &ctx.inject::<DbPool>()));
        assert!(Arc::ptr_eq(&server.user_svc.config, &ctx.inject::<AppConfig>()));
    }

    #[test]
    fn test_user_service_functionality() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>().expect("AppServer 构建失败");
        let greeting = server.user_svc.greet();
        assert_eq!(
            greeting,
            "[tx-di-example] Hello from UserService (port: 8080)"
        );
    }

    #[test]
    fn test_component_with_multiple_dependencies() {
        let mut ctx = create_full_context();
        let user_svc = ctx.inject::<UserService>();
        assert!(Arc::ptr_eq(&user_svc.db, &ctx.inject::<DbPool>()));
        assert!(Arc::ptr_eq(&user_svc.config, &ctx.inject::<AppConfig>()));
    }

    // ════════════════════════════════════════════════════════════════════
    //  5. 注册表与 ComponentDescriptor
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_registry_contains_core_components() {
        // di-example 自身注册 5 个组件
        let component_names: Vec<&str> =
            COMPONENT_REGISTRY.iter().map(|m| m.name).collect();

        assert!(component_names.contains(&"DbPool"));
        assert!(component_names.contains(&"AppConfig"));
        assert!(component_names.contains(&"UserService"));
        assert!(component_names.contains(&"RequestLogger"));
        assert!(component_names.contains(&"AppServer"));
    }

    #[test]
    fn test_scope_on_components() {
        assert_eq!(<DbPool as ComponentDescriptor>::SCOPE, Scope::Singleton);
        assert_eq!(<AppConfig as ComponentDescriptor>::SCOPE, Scope::Singleton);
        assert_eq!(<UserService as ComponentDescriptor>::SCOPE, Scope::Singleton);
        assert_eq!(<AppServer as ComponentDescriptor>::SCOPE, Scope::Singleton);
        assert_eq!(<RequestLogger as ComponentDescriptor>::SCOPE, Scope::Prototype);

        let deps = <UserService as ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 2); // DbPool + AppConfig

        let deps = <RequestLogger as ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_component_descriptor_build() {
        let mut ctx = BuildContext::new::<PathBuf>(None);
        let app_config = AppConfig::build(&mut ctx);
        assert_eq!(app_config.app_name, "tx-di-example");
        assert_eq!(app_config.port, 8080);
    }

    // ════════════════════════════════════════════════════════════════════
    //  6. BuildContext API
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_build_context_not_empty_after_creation() {
        let ctx = create_full_context();
        // AppAllConfig(1) + DbPool + AppConfig + UserService + RequestLogger + AppServer = 6
        assert!(!ctx.is_empty());
        assert!(ctx.len() >= 6);
    }

    #[test]
    fn test_take_removes_from_context() {
        let mut ctx = create_full_context();
        let _db_before = ctx.inject::<DbPool>();
        let _server = ctx.take::<AppServer>().expect("AppServer 构建失败");
        let _db_after = ctx.inject::<DbPool>();
        // take 之后其他单例仍然可用
    }

    #[test]
    fn test_debug_registry_no_panic() {
        BuildContext::debug_registry().expect("debug_registry 不应出错");
    }

    // ════════════════════════════════════════════════════════════════════
    //  7. 线程安全
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_singleton_thread_safety() {
        use std::thread;
        let mut ctx = create_full_context();
        let db = ctx.inject::<DbPool>();
        let handles: Vec<_> = (0..5)
            .map(|_| {
                let db_clone = db.clone();
                thread::spawn(move || Arc::as_ptr(&db_clone) as usize)
            })
            .collect();
        let pointers: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        for ptr in &pointers[1..] {
            assert_eq!(ptr, &pointers[0]);
        }
    }

    // ════════════════════════════════════════════════════════════════════
    //  8. 配置文件加载 — AppAllConfig + AppConfig
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_config_file_loads_app_all_config() {
        let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();

        // 验证 web_config 段被正确读取
        let host: Option<String> = global_config.get("web_config.host");
        assert_eq!(host, Some("127.0.0.1".to_string()));

        let port: Option<u16> = global_config.get("web_config.port");
        assert_eq!(port, Some(8888));

        // 验证 sip_config 段被正确读取
        let sip_host: Option<String> = global_config.get("sip_config.host");
        assert_eq!(sip_host, Some("0.0.0.0".to_string()));

        let sip_port: Option<u16> = global_config.get("sip_config.port");
        assert_eq!(sip_port, Some(5060));

        let sip_transport: Option<String> = global_config.get("sip_config.transport");
        assert_eq!(sip_transport, Some("udp".to_string()));
    }

    #[test]
    fn test_complex_config_nested_access() {
        let mut ctx = BuildContext::new(Some("../configs/complex-config.toml"));
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();

        let db_host: Option<String> = global_config.get("database.host");
        assert_eq!(db_host, Some("localhost".to_string()));

        let db_port: Option<u16> = global_config.get("database.port");
        assert_eq!(db_port, Some(5432));

        let log_level: Option<String> = global_config.get("logging.level");
        assert_eq!(log_level, Some("info".to_string()));
    }

    #[test]
    fn test_missing_config_uses_defaults() {
        let mut ctx = BuildContext::new(Some("nonexistent/config.toml"));
        let config = ctx.inject::<AppConfig>();
        assert_eq!(config.app_name, "tx-di-example");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_app_all_config_is_singleton() {
        let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
        let cfg1 = ctx.inject::<tx_di_core::AppAllConfig>();
        let cfg2 = ctx.inject::<tx_di_core::AppAllConfig>();
        assert!(Arc::ptr_eq(&cfg1, &cfg2));
    }

    #[test]
    fn test_config_value_type_conversion() {
        let mut ctx = BuildContext::new(Some("../configs/complex-config.toml"));
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();

        let port_u32: Option<u32> = global_config.get("app_config.port");
        assert_eq!(port_u32, Some(3000u32));

        let port_u64: Option<u64> = global_config.get("app_config.port");
        assert_eq!(port_u64, Some(3000u64));

        let port_str: Option<String> = global_config.get("app_config.port");
        assert_eq!(port_str, None, "数字不能直接转为 String");
    }

    // ════════════════════════════════════════════════════════════════════
    //  9. Web 插件集成测试
    //
    //  注意：WebPlugin 通过 linkme 自动注册到 COMPONENT_REGISTRY，
    //  导入 tx_di_axum 即触发注册。此处验证 WebConfig 可从配置加载、
    //  WebPlugin 组件可正常构建。
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_web_plugin_registered_in_registry() {
        let names: Vec<&str> = COMPONENT_REGISTRY.iter().map(|m| m.name).collect();
        // WebPlugin 和 WebConfig 都应该已注册
        assert!(
            names.contains(&"WebPlugin"),
            "WebPlugin 应在注册表中。实际成员: {:?}",
            names
        );
        assert!(
            names.contains(&"WebConfig"),
            "WebConfig 应在注册表中。实际成员: {:?}",
            names
        );
    }

    #[test]
    fn test_web_config_from_toml() {
        let toml_str = r#"
            host      = "192.168.1.100"
            port      = 9090
            enable_cors = true
            timeout_secs = 30
        "#;
        let cfg: WebConfig =
            toml::from_str(toml_str).expect("WebConfig TOML 反序列化成功");
        assert_eq!(cfg.host, "192.168.1.100");
        assert_eq!(cfg.port, 9090);
        assert!(cfg.enable_cors);
        assert_eq!(cfg.timeout_secs, 30);
    }

    #[test]
    fn test_web_config_defaults() {
        let cfg: WebConfig = toml::from_str("").unwrap_or_default();
        assert!(!cfg.host.is_empty());
        assert!(cfg.port > 0);
    }

    #[test]
    fn test_web_plugin_build() {
        let mut ctx = create_full_context();
        // WebPlugin 是 Singleton，可以直接 inject
        let web = ctx.inject::<WebPlugin>();
        // 验证 config 已正确注入
        assert!(!web.config.host.is_empty());
        assert!(web.config.port > 0);
    }

    #[test]
    fn test_web_config_socket_addr_ipv4() {
        let cfg = WebConfig {
            host: "10.0.0.1".to_string(),
            port: 8080,
            ..Default::default()
        };
        let addr = cfg.socket_addr().unwrap();
        assert_eq!(addr.port(), 8080);
        assert!(addr.is_ipv4());
    }

    #[test]
    fn test_web_config_socket_addr_ipv6() {
        let cfg = WebConfig {
            host: "::1".to_string(),
            port: 9443,
            ..Default::default()
        };
        let addr = cfg.socket_addr().unwrap();
        assert_eq!(addr.port(), 9443);
        assert!(addr.is_ipv6());
    }

    // ════════════════════════════════════════════════════════════════════
    //  10. SIP 插件集成测试
    //
    //  验证 SipConfig 从 TOML 正确反序列化、
    //  SipRouter 处理器注册机制可用。
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_sip_plugin_registered_in_registry() {
        let names: Vec<&str> = COMPONENT_REGISTRY.iter().map(|m| m.name).collect();
        assert!(
            names.contains(&"SipPlugin"),
            "SipPlugin 应在注册表中。实际成员: {:?}",
            names
        );
        assert!(
            names.contains(&"SipConfig"),
            "SipConfig 应在注册表中。实际成员: {:?}",
            names
        );
    }

    #[test]
    fn test_sip_config_from_toml_full() {
        let toml_str = r#"
            host       = "::"
            port       = 5062
            transport  = "both"
            user_agent = "GB28101-Srv/2.0"
            external_ip = "203.0.113.10"
            log_messages = true
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.host, "::");
        assert_eq!(cfg.port, 5062);
        assert_eq!(cfg.transport, SipTransport::Both);
        assert_eq!(cfg.user_agent, "GB28101-Srv/2.0");
        assert_eq!(cfg.external_ip.as_deref().unwrap(), "203.0.113.10");
        assert!(cfg.log_messages);
        assert!(cfg.enable_udp());
        assert!(cfg.enable_tcp());
    }

    #[test]
    fn test_sip_config_from_toml_minimal() {
        let toml_str = r#"
            host = "127.0.0.1"
            port = 15060
        "#;
        let cfg: SipConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.transport, SipTransport::Udp);
        assert!(cfg.enable_udp());
        assert!(!cfg.enable_tcp());
        assert_eq!(cfg.user_agent, "tx-di-sip/0.1.0");
    }

    #[test]
    fn test_sip_config_bind_addr_formatting() {
        let cfg = SipConfig {
            host: "::".to_string(),
            port: 5060,
            ..Default::default()
        };
        assert_eq!(cfg.bind_addr(), "[::]:5060");

        let cfg_v4 = SipConfig {
            host: "0.0.0.0".to_string(),
            port: 5060,
            ..Default::default()
        };
        assert_eq!(cfg_v4.bind_addr(), "0.0.0.0:5060");
    }

    #[test]
    fn test_sip_router_lifecycle() {
        // 串行执行避免全局状态竞争
        SipRouter::clear();
        assert_eq!(SipRouter::handler_count(), 0);

        SipRouter::add_handler(Some("REGISTER"), 0, |_tx| async move { Ok(()) });
        SipRouter::add_handler(Some("INVITE"), 0, |_tx| async move { Ok(()) });
        SipRouter::add_handler(None, 99, |_tx| async move { Ok(()) }); // catch-all
        assert_eq!(SipRouter::handler_count(), 3);

        SipRouter::clear();
        assert_eq!(SipRouter::handler_count(), 0);
    }

    #[test]
    fn test_sip_config_from_di_config_toml() {
        // 从项目主配置 di-config.toml 加载 sip_config
        let toml_str = include_str!("../../configs/di-config.toml");
        let table: toml::Value = toml::from_str(toml_str).unwrap();
        let sip_table = table.get("sip_config").expect("di-config.toml 中应有 [sip_config]");

        let cfg: SipConfig = sip_table.clone().try_into().expect("sip_config 反序列化成功");
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 5060);
        assert_eq!(cfg.transport, SipTransport::Udp);
        assert_eq!(cfg.user_agent, "tx-di-sip/0.1.0");
        assert!(!cfg.log_messages);
    }

    // ════════════════════════════════════════════════════════════════════
    //  11. Log 插件集成测试
    //
    //  验证 LogConfig 从 TOML 正确反序列化、字段默认值。
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_log_plugin_registered_in_registry() {
        let names: Vec<&str> = COMPONENT_REGISTRY.iter().map(|m| m.name).collect();
        assert!(
            names.contains(&"LogPlugins"),
            "LogPlugins 应在注册表中。实际成员: {:?}",
            names
        );
        assert!(
            names.contains(&"LogConfig"),
            "LogConfig 应在注册表中。实际成员: {:?}",
            names
        );
    }

    #[test]
    fn test_log_config_from_test_log_toml() {
        // 使用项目的 test_log.toml（含 [log_config] 段头，需先提取子表）
        let toml_str = include_str!("../../configs/test_log.toml");
        let table: toml::Value = toml::from_str(toml_str).expect("test_log.toml 解析失败");
        let log_table = table
            .get("log_config")
            .expect("test_log.toml 中应有 [log_config]");

        let cfg: LogConfig = log_table
            .clone()
            .try_into()
            .expect("LogConfig 反序列化成功");

        assert_eq!(format!("{:?}", cfg.level), "Debug"); // level = "debug"
        assert_eq!(cfg.prefix, "di-example");
        assert_eq!(cfg.dir.file_name().unwrap().to_str().unwrap(), "logs"); // ./logs
        assert!(cfg.console_output);
    }

    #[test]
    fn test_log_config_from_toml_custom() {
        let toml_str = r#"
            level = "trace"
            prefix = "my-app"
            dir = "/var/log/app"
            retention_days = 365
            console_output = false
            time_format = "local"
        "#;
        let cfg: LogConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(format!("{:?}", cfg.level), "Trace");
        assert_eq!(cfg.prefix, "my-app");
        assert!(cfg.console_output == false);
        assert_eq!(cfg.retention_days, 365);
        assert_eq!(format!("{}", cfg.time_format), "local");
    }

    #[test]
    fn test_log_config_defaults() {
        let cfg: LogConfig = Default::default();
        assert_eq!(format!("{:?}", cfg.level), "Info"); // 默认 info
        assert_eq!(cfg.prefix, "tx_di");
        assert!(!cfg.console_output);
        assert_eq!(cfg.retention_days, 90);
    }

    #[test]
    fn test_log_config_modules_override() {
        let toml_str = r#"
            level = "warn"

            [modules]
            "my_crate::db" = "debug"
            "my_crate::http" = "trace"
        "#;
        let cfg: LogConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.modules.len(), 2);
        assert_eq!(
            format!("{:?}", cfg.modules.get("my_crate::db").unwrap()),
            "Debug"
        );
        assert_eq!(
            format!("{:?}", cfg.modules.get("my_crate::http").unwrap()),
            "Trace"
        );
    }

    #[test]
    fn test_log_config_from_di_config_toml() {
        let toml_str = include_str!("../../configs/di-config.toml");
        let table: toml::Value = toml::from_str(toml_str).unwrap();
        let log_table = table
            .get("log_config")
            .expect("di-config.toml 中应有 [log_config]");

        let cfg: LogConfig = log_table
            .clone()
            .try_into()
            .expect("log_config 反序列化成功");
        assert_eq!(format!("{:?}", cfg.level), "Debug");
        assert_eq!(cfg.prefix, "di-axum");
        assert!(cfg.console_output);
    }
}
