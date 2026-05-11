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

use tx_di_core::{tx_comp, BoxFuture, BuildContext, CompInit, App, RIE, InnerContext};
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
    fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> {
        info!("AppServer::inner_init");
        Ok(())
    }
    fn init(_ctx: Arc<App>,_token: CancellationToken) -> RIE<()> {
        debug!("AppServer::init");
        Ok(())
    }
    fn async_init(ctx: Arc<App>,_token: CancellationToken) -> BoxFuture {
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
    use std::any::TypeId;

    use tx_di_core::{ComponentDescriptor, Scope, COMPONENT_REGISTRY};
    /// 会自动注册axum
    // use tx_di_axum::{WebConfig, WebPlugin};
    /// 会自动注册sip
    // use tx_di_sip::{SipConfig, SipRouter, SipTransport};
    /// 会自动注册log,且优先级第一，确保log组件优先初始化
    #[allow(unused)]
    use tx_di_log;
    use super::*;

    /// 辅助函数：创建无配置文件的上下文（自动扫描所有组件）
    fn create_full_context() -> BuildContext {
        // C:\a_me\proj\rust\tx_di\configs\di-example.toml
        // D:\proj\tx_di\configs\di-example.toml
        BuildContext::new(Some(r"D:\proj\tx_di\configs\di-example.toml"))
    }

    // ════════════════════════════════════════════════════════════════════
    //  1. DI 核心功能 — 单例
    // ════════════════════════════════════════════════════════════════════

    /// 单例注入
    #[test]
    fn test_singleton_shared() ->RIE<()> {
        let  app = create_full_context().build()?;
        // debug!("AppServer::init");
        // 获取单例
        let server = app.inject::<AppServer>();
        // 获取可能不存在的单例
        let opt_server = app.try_inject::<AppServer>();
        // UserService 内的 DbPool 和 ctx 里的是同一个 Arc
        let db_in_ctx = app.inject::<DbPool>();
        assert!(
            Arc::ptr_eq(&server.user_svc.db, &db_in_ctx),
            "DbPool 应该是同一个 Arc 实例（单例）"
        );
        assert!(
            opt_server.is_some(),
            "AppServer::try_inject() 返回 Some(AppServer)，单例已存在"
        );
        if let Some(server) = opt_server {
            assert!(
                Arc::ptr_eq(&server.user_svc.db, &db_in_ctx),
                "DbPool 应该是同一个 Arc 实例（单例）"
            );
            assert!(
                Arc::ptr_eq(&server, &server),
                "AppServer 应该是同一个"
            );
        }
        Ok(())
    }

    /// 多次注入同一单例实例
    #[test]
    fn test_singleton_multiple_injects_same_instance() -> RIE<()> {
        let ctx = create_full_context().build()?;
        let db1 = ctx.inject::<DbPool>();
        let db2 = ctx.inject::<DbPool>();
        let db3 = ctx.inject::<DbPool>();
        let db4 = ctx.try_inject::<DbPool>();
        assert!(Arc::ptr_eq(&db1, &db2), "两次注入应该返回相同实例");
        assert!(Arc::ptr_eq(&db2, &db3), "三次注入应该返回相同实例");
        assert!(db4.is_some(), "try_inject() 返回 Some(DbPool)");
        assert!(Arc::ptr_eq(&db4.unwrap(), &db1), "try_inject() 返回的实例应该和单例相同");
        Ok(())
    }

    #[test]
    fn test_singleton_arc_clone_shares_data() {
        let ctx = create_full_context().build().unwrap();
        let db1 = ctx.inject::<DbPool>();
        let _db2 = db1.clone();
        // 引用计数: ctx(1) + db1 + clone = 至少 3
        assert!(Arc::strong_count(&db1) >= 3);
    }

    // ════════════════════════════════════════════════════════════════════
    //  2. DI 核心功能 — 原型 (Prototype)
    // ════════════════════════════════════════════════════════════════════

    /// 验证 Prototype 组件每次注入产生独立实例
    #[test]
    fn test_prototype_independent() {
        let ctx = create_full_context().build().unwrap();
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

    /// 验证：build() 后的 App 也能注入 Prototype 组件
    #[test]
    fn test_prototype_inject_from_app() {
        let app = create_full_context().build().unwrap();

        // 从 App 注入 Prototype，不应 panic
        let l1 = app.inject::<RequestLogger>();
        let l2 = app.inject::<RequestLogger>();

        // 每次注入创建新实例
        assert_ne!(
            Arc::as_ptr(&l1),
            Arc::as_ptr(&l2),
            "App 阶段 Prototype 注入应产生不同实例"
        );
        // 各实例状态独立
        assert_eq!(l1.count(), 0);
        assert_eq!(l2.count(), 0);

        // 验证 try_inject 也能拿到 Prototype
        let l3 = app.try_inject::<RequestLogger>();
        assert!(l3.is_some(), "App::try_inject 应能返回 Prototype");
    }

    /// 验证：App::inject 返回的 Prototype 中注入的 Singleton 仍是全局同一实例
    #[test]
    fn test_prototype_from_app_uses_same_singletons() {
        let app = create_full_context().build().unwrap();

        // Singleton：两次注入指向同一实例
        let db1 = app.inject::<DbPool>();
        let db2 = app.inject::<DbPool>();
        assert!(Arc::ptr_eq(&db1, &db2), "Singleton 在 App 阶段应返回同一实例");

        // Prototype 中包含 Singleton 依赖，验证也是同一个
        let server = app.inject::<AppServer>();
        assert!(
            Arc::ptr_eq(&server.user_svc.db, &db1),
            "Prototype 注入的 Singleton 应与全局一致"
        );
    }

    #[test]
    fn test_prototype_each_inject_creates_new_instance() {
        let ctx = create_full_context();
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
        let ctx = create_full_context();
        let logger = ctx.inject::<RequestLogger>();
        assert_eq!(logger.prefix, "[REQUEST]");
        assert_eq!(logger.count(), 0);
    }

    #[test]
    fn test_prototype_state_isolation() {
        let ctx = create_full_context();
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
        let ctx = create_full_context().build().unwrap();
        let server = ctx.inject::<AppServer>();
        assert_eq!(server.bind_addr, "0.0.0.0:8080");
        assert_eq!(
            server.default_headers.get("X-Powered-By").unwrap(),
            "di-framework"
        );
    }

    /// 测试配置文件值和默认值
    #[test]
    fn test_app_config_default_values() {
        let ctx = create_full_context().build().unwrap();
        let cfg = ctx.inject::<AppConfig>();
        assert_eq!(cfg.app_name, "zdy_name");
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn test_custom_value_expression_evaluated_once() {
        let ctx = create_full_context().build().unwrap();
        let cfg1 = ctx.inject::<AppConfig>();
        let cfg2 = ctx.inject::<AppConfig>();
        assert!(Arc::ptr_eq(&cfg1, &cfg2));
    }

    // ════════════════════════════════════════════════════════════════════
    //  4. 依赖链
    // ════════════════════════════════════════════════════════════════════

    /// 测试依赖注入链
    #[test]
    fn test_dependency_injection_chain() {
        let ctx = create_full_context().build().unwrap();
        let server = ctx.try_inject::<AppServer>().unwrap();
        assert!(Arc::ptr_eq(&server.user_svc.db, &ctx.inject::<DbPool>()));
        assert!(Arc::ptr_eq(&server.user_svc.config, &ctx.inject::<AppConfig>()));
    }

    /// 测试 UserService 功能
    #[test]
    fn test_user_service_functionality() {
        let ctx = create_full_context().build().unwrap();
        let server = ctx.inject::<AppServer>();
        let greeting = server.user_svc.greet();
        assert_eq!(
            greeting,
            "[zdy_name] Hello from UserService (port: 8080)"
        );
    }

    /// 测试 UserService 组件
    #[test]
    fn test_component_with_multiple_dependencies() {
        let ctx = create_full_context().build().unwrap();
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
        // 将 DEP_IDS 中的函数调用结果转换为 Vec<TypeId>，然后检查是否包含目标 TypeId
        let dep_type_ids: Vec<TypeId> = deps.iter().map(|f| f()).collect();
        assert!(dep_type_ids.contains(&TypeId::of::<DbPool>()));
        assert!(dep_type_ids.contains(&TypeId::of::<AppConfig>()));

        let deps = <RequestLogger as ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 0);
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
    fn test_debug_registry_no_panic() {
        BuildContext::debug_registry().expect("debug_registry 不应出错");
    }

    // ════════════════════════════════════════════════════════════════════
    //  7. 线程安全
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_singleton_thread_safety() {
        use std::thread;
        let ctx = create_full_context().build().unwrap();
        let db = ctx.inject::<DbPool>();
        let handles: Vec<_> = (0..5000)
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
        let ctx = create_full_context().build().unwrap();
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();

        // 验证 web_config 段被正确读取
        let host: Option<String> = global_config.get("web_config.host");
        assert_eq!(host, Some("192.168.0.140".to_string()));

        let port: Option<u16> = global_config.get("web_config.port");
        assert_eq!(port, Some(8089));

        // 验证 sip_config 段被正确读取
        let sip_host: Option<String> = global_config.get("sip_config.host");
        assert_eq!(sip_host, Some("::".to_string()));

        let sip_port: Option<u16> = global_config.get("sip_config.port");
        assert_eq!(sip_port, Some(5069));

        let sip_transport: Option<String> = global_config.get("sip_config.transport");
        assert_eq!(sip_transport, Some("both".to_string()));
    }

    /// 测试嵌套结构
    #[test]
    fn test_complex_config_nested_access() {
        let ctx = create_full_context().build().unwrap();
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();

        let db_host: Option<String> = global_config.get("database.host");
        assert_eq!(db_host, Some("localhost".to_string()));

        let db_port: Option<u16> = global_config.get("database.port");
        assert_eq!(db_port, Some(5432));

        let log_level: Option<String> = global_config.get("log_config.level");
        assert_eq!(log_level, Some("debug".to_string()));
    }

    #[test]
    fn test_missing_config_uses_defaults() {
        let ctx = BuildContext::new(Some("nonexistent/config.toml"));
        let config = ctx.inject::<AppConfig>();
        assert_eq!(config.app_name, "tx-di-example");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_app_all_config_is_singleton() {
        let ctx = create_full_context().build().unwrap();
        let cfg1 = ctx.inject::<tx_di_core::AppAllConfig>();
        let cfg2 = ctx.inject::<tx_di_core::AppAllConfig>();
        assert!(Arc::ptr_eq(&cfg1, &cfg2));
    }

    #[test]
    fn test_config_value_type_conversion() {
        let ctx = create_full_context().build().unwrap();
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();

        let port_u32: Option<u32> = global_config.get("web_config.port");
        assert_eq!(port_u32, Some(8089u32));

        let port_u64: Option<u64> = global_config.get("web_config.port");
        assert_eq!(port_u64, Some(8089u64));

        let port_str: Option<String> = global_config.get("web_config.port");
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
    fn test_web_plugin_build() {
        use tx_di_axum::WebPlugin;
        let ctx = create_full_context().build().unwrap();
        // WebPlugin 是 Singleton，可以直接 inject
        let web = ctx.inject::<WebPlugin>();
        // 验证 config 已正确注入
        assert_eq!(web.config.host, "192.168.0.140","host 配置和配置文件不同");
        assert_eq!(web.config.port,8089,"port 配置和配置文件不同");
        assert_eq!(web.config.enable_cors, true);
        // 默认值
        assert_eq!(web.config.max_body_size, 1024 * 1024 * 10);

    }

    #[test]
    fn test_web_config_socket_addr_ipv4() {
        use tx_di_axum::*;
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
        use tx_di_axum::*;
        let cfg = WebConfig {
            host: "::1".to_string(),
            port: 9443,
            ..Default::default()
        };
        let addr = cfg.socket_addr().unwrap();
        assert_eq!(addr.port(), 9443);
        assert!(addr.is_ipv6());
    }

    /// todo web 功能测试
    #[tokio::test]
    async fn test_web_plugin_start() {
        // 使用api工具测试
    }

    // ════════════════════════════════════════════════════════════════════
    //  10. SIP 插件集成测试
    //  todo sip 测试
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
    fn test_sip_router_lifecycle() {
        use tx_di_sip::*;
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

    #[tokio::test]
    async fn test_sip_config_from_di_example_toml() {
        use tx_di_sip::*;
        let ctx = create_full_context().build().unwrap();
        let config = ctx.inject::<SipConfig>();
        assert_eq!(config.port, 5069, "端口和配置文件不同");
        assert_eq!(config.user_agent, "tx-di-sip/0.1.0", "user agent 和配置文件不同");
    }

    // ════════════════════════════════════════════════════════════════════
    //  11. Log 插件集成测试
    // todo log 测试
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_log_plugin_registered_in_registry() {
        #[allow(unused)]
        use tx_di_log;
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
}
