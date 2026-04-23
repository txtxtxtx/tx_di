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
//!  ┌──────────┐   ┌────────────┐
//!  │  DbPool  │   │ AppConfig  │  ← #[tx_comp]（默认 Singleton）
//!  └────┬─────┘   └─────┬──────┘
//!       │                    │
//!       ▼ Singleton          ▼ Singleton (自动)
//!  ┌──────────────────┐  ┌──────────────────────┐
//!  │    UserService   │  │    RequestLogger     │← #[tx_comp(scope = Prototype)]
//!  │  db: Arc<DbPool> │  └──────────┬───────────┘
//!  │  config: Arc<..> │              │ Prototype (自动)
//!  └────────┬─────────┘              │
//!           │ Singleton               │
//!           └──────────┬──────────────┘
//!                      ▼
//!              ┌──────────────┐
//!              │  AppServer   │
//!              │  user_svc    │
//!              │  logger      │ ← Arc<RequestLogger>，每次注入新实例
//!              │  extra       │ ← #[tx_cst(expr)]
//!              └──────────────┘
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tx_di_core::{tx_comp, BoxFuture, BuildContext, CompInit, IE, RIE};
use log::{debug, info};
use serde::Deserialize;
use tracing::error;
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
    fn async_init(ctx: &mut BuildContext) -> BoxFuture<'static, RIE<()>> {
        let len = ctx.len();
        Box::pin(async move {
            debug!("AppServer::async_init:{}",len);
            Ok(())
        })
    }
    fn init(ctx: &mut BuildContext) ->RIE<()> {
        debug!("AppServer::init:{}",ctx.len());
        Ok(())
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
// 5. main
// ─────────────────────────────────────────────────────────────────────────────
use tx_di_log;
use tx_di_axum;  // 导入 web 插件以触发组件注册

#[tokio::main]
async fn main() {
    run().await.map_err(|e| error!("{}", e)).expect("启动失败")
}

async fn run() ->RIE<()> {
    
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    info!("🚀 tx_di 启动");

    // 方式 1：自动扫描所有注册的组件（无需配置文件）
    let mut ctx = BuildContext::new(Some("configs/test_log.toml"));
    
    // 方式 2：从配置文件加载指定组件（取消注释使用）
    // let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
    
    ctx.run().await.expect("TODO: panic message");
    info!("构建完成");
    
    // WebPlugin 已自动启动 web 服务器
    // 可以通过 http://127.0.0.1:8080/health 访问健康检查端点
    // ── 取出 AppServer ──────────────────────────────────────────────────
    let server = ctx.take::<AppServer>()?;
    info!("✅ AppServer 构建完成");
    info!("   bind_addr  = {}", server.bind_addr);
    info!("   headers    = {:?}", server.default_headers);

    // ── 验证单例共享 ────────────────────────────────────────────────────
    let greeting = server.user_svc.greet();
    info!("   UserService.greet() = {}", greeting);

    // server.user_svc.db 是 Arc<DbPool>，Arc derefs → 可以直接比较指针
    let db_in_ctx = ctx.inject::<DbPool>();
    let db_in_ctx_ptr = Arc::as_ptr(&db_in_ctx);
    println!(
        "   DbPool Arc 共享 = {}（{} vs {}）",
        if Arc::as_ptr(&server.user_svc.db) == db_in_ctx_ptr {
            "✅ 同实例"
        } else {
            "❌ 不同实例"
        },
        Arc::as_ptr(&server.user_svc.db) as usize,
        db_in_ctx_ptr as usize
    );

    // ── 验证原型独立性 ─────────────────────────────────────────────────
    println!("\n🔄 验证 Prototype 独立性：");
    server.logger.log("第一条日志");
    server.logger.log("第二条日志");

    // 从 ctx 再次 inject Prototype 组件 → 构造新实例
    let another_logger = ctx.inject::<RequestLogger>();
    another_logger.log("另一个 logger 的第一条");
    info!("   another_logger.count = {}（独立，从 0 开始）", another_logger.count());
    info!("   server.logger.count = {}（独立）", server.logger.count());

    // ── 验证 AppConfig ─────────────────────────────────────────────────
    info!("\n⚙️  AppConfig（#[tx_cst] 自定义值）：");
    let cfg_arc = ctx.inject::<AppConfig>();
    info!("   app_name = {}", cfg_arc.app_name);
    info!("   port     = {}", cfg_arc.port);
    
    // ── 验证全局配置对象 ───────────────────────────────────────────────
    info!("\n📄 验证全局配置对象（AppAllConfig）：");
    let global_config = ctx.inject::<tx_di_core::AppAllConfig>();
    if let Some(app_name) = global_config.get::<String>("app_config.app_name") {
        info!("   从配置读取 app_name: {}", app_name);
    }
    if let Some(port) = global_config.get::<u16>("app_config.port") {
        info!("   从配置读取 port: {}", port);
    }
    
    BuildContext::debug_registry();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试 todo
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use tx_di_core::ComponentDescriptor;
    use super::*;

    /// 辅助函数：创建包含所有组件的上下文
    fn create_full_context() -> BuildContext {
        BuildContext::new::<PathBuf>(None)
    }

    // ── 单例测试 ───────────────────────────────────────────────────────
    /// 单例测试
    #[test]
    fn test_singleton_shared() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>();

        // UserService 内的 DbPool 和 ctx 里的是同一个 Arc
        let db_in_ctx = ctx.inject::<DbPool>();
        assert!(
            std::sync::Arc::ptr_eq(&server.user_svc.db, &db_in_ctx),
            "DbPool 应该是同一个 Arc 实例（单例）"
        );
    }
    /// 多次注入单例应该返回相同的 Arc
    #[test]
    fn test_singleton_multiple_injects_same_instance() {
        let mut ctx = create_full_context();

        // 多次注入单例应该返回相同的 Arc 实例
        let db1 = ctx.inject::<DbPool>();
        let db2 = ctx.inject::<DbPool>();
        let db3 = ctx.inject::<DbPool>();

        assert!(Arc::ptr_eq(&db1, &db2), "两次注入应该返回相同实例");
        assert!(Arc::ptr_eq(&db2, &db3), "三次注入应该返回相同实例");
    }
    
    /// Arc clone() 方法应该共享数据
    #[test]
    fn test_singleton_arc_clone_shares_data() {
        let mut ctx = create_full_context();

        let db1 = ctx.inject::<DbPool>();
        let db2 = db1.clone();
        // 上下文持有一个,
        // AppServer 持有一个
        // clone 只是增加引用计数，不创建新实例
        assert!(Arc::ptr_eq(&db1, &db2));
        assert_eq!(Arc::strong_count(&db1), 4);
    }

    // ── 原型测试 ───────────────────────────────────────────────────────
    /// 原型测试
    #[test]
    fn test_prototype_independent() {
        let mut ctx = create_full_context();

        // 两次 inject Prototype 组件 → 构造两个独立实例
        let l1 = ctx.inject::<RequestLogger>();
        let l2 = ctx.inject::<RequestLogger>();

        l1.log("l1 msg");
        l2.log("l2 msg");

        // 两个 logger 的计数器相互独立
        assert_ne!(
            Arc::as_ptr(&l1),
            Arc::as_ptr(&l2),
            "Prototype 实例应该相互独立（不同 Arc 指针）"
        );
        assert_eq!(l1.count(), 1);
        assert_eq!(l2.count(), 1, "每个 logger 计数器独立");
    }
    /// 每次 inject 创建新实例
    #[test]
    fn test_prototype_each_inject_creates_new_instance() {
        let mut ctx = create_full_context();

        // 每次 inject 都应该创建新实例
        let loggers: Vec<_> = (0..5).map(|_| ctx.inject::<RequestLogger>()).collect();

        // 所有实例的指针都应该不同
        for i in 0..loggers.len() {
            for j in (i + 1)..loggers.len() {
                assert_ne!(
                    Arc::as_ptr(&loggers[i]),
                    Arc::as_ptr(&loggers[j]),
                    "第 {} 和第 {} 次注入应该产生不同实例",
                    i + 1,
                    j + 1
                );
            }
        }
    }
    /// 自定义值注入测试
    #[test]
    fn test_prototype_with_custom_values() {
        let mut ctx = create_full_context();

        let logger = ctx.inject::<RequestLogger>();

        // 验证 #[tx_cst] 注入的自定义值正确
        assert_eq!(logger.prefix, "[REQUEST]");
        assert_eq!(logger.count(), 0, "新实例的计数器应该从 0 开始");
    }

    // ── 自定义值注入测试 ───────────────────────────────────────────────
    /// 自定义值注入测试
    #[test]
    fn test_inject_custom_values() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>();

        assert_eq!(server.bind_addr, "0.0.0.0:8080");
        assert_eq!(
            server.default_headers.get("X-Powered-By").unwrap(),
            "di-framework"
        );
    }
    /// 自定义值注入测试
    #[test]
    fn test_app_config_inject() {
        let mut ctx = create_full_context();
        let cfg = ctx.inject::<AppConfig>();

        assert_eq!(cfg.app_name, "tx-di-example");
        assert_eq!(cfg.port, 8080);
    }
    /// 自定义值表达式求值一次
    #[test]
    fn test_custom_value_expression_evaluated_once() {
        let mut ctx = create_full_context();

        // 对于单例组件，#[tx_cst] 表达式只在构建时求值一次
        let cfg1 = ctx.inject::<AppConfig>();
        let cfg2 = ctx.inject::<AppConfig>();

        // 两个引用指向同一个实例
        assert!(Arc::ptr_eq(&cfg1, &cfg2));
    }

    // ── 依赖关系测试 ───────────────────────────────────────────────────
    /// 依赖关系测试
    #[test]
    fn test_dependency_injection_chain() {
        let mut ctx = create_full_context();

        // AppServer 依赖 UserService
        let server = ctx.take::<AppServer>();

        // UserService 依赖 DbPool 和 AppConfig
        assert!(Arc::ptr_eq(&server.user_svc.db, &ctx.inject::<DbPool>()));
        assert!(Arc::ptr_eq(&server.user_svc.config, &ctx.inject::<AppConfig>()));
    }
    /// 验证 UserService 功能
    #[test]
    fn test_user_service_functionality() {
        let mut ctx = create_full_context();
        let server = ctx.take::<AppServer>();

        // 验证 UserService 可以正常使用注入的依赖
        let greeting = server.user_svc.greet();
        assert_eq!(greeting, "[tx-di-example] Hello from UserService (port: 8080)");
    }

    // ── 注册表测试 ─────────────────────────────────────────────────────
    /// 注册表测试
    #[test]
    fn test_registry() {
        let count = tx_di_core::COMPONENT_REGISTRY.len();
        assert_eq!(count, 5, "应该有 5 个注册组件（不包括 AppAllConfig）");
        
        // 验证每个组件都存在
        let component_names: Vec<&str> = tx_di_core::COMPONENT_REGISTRY.iter()
            .map(|m| m.name)
            .collect();
        
        assert!(component_names.contains(&"DbPool"));
        assert!(component_names.contains(&"AppConfig"));
        assert!(component_names.contains(&"UserService"));
        assert!(component_names.contains(&"RequestLogger"));
        assert!(component_names.contains(&"AppServer"));
    }
    /// 验证 scope
    #[test]
    fn test_scope_on_component() {
        // 验证 scope 确实在组件自己身上
        assert_eq!(
            tx_di_core::Scope::Singleton,
            <DbPool as tx_di_core::ComponentDescriptor>::SCOPE
        );
        assert_eq!(
            tx_di_core::Scope::Singleton,
            <AppConfig as tx_di_core::ComponentDescriptor>::SCOPE
        );
        assert_eq!(
            tx_di_core::Scope::Singleton,
            <UserService as tx_di_core::ComponentDescriptor>::SCOPE
        );
        assert_eq!(
            tx_di_core::Scope::Singleton,
            <AppServer as tx_di_core::ComponentDescriptor>::SCOPE
        );
        assert_eq!(
            tx_di_core::Scope::Prototype,
            <RequestLogger as tx_di_core::ComponentDescriptor>::SCOPE
        );

        // UserService 依赖 DbPool 和 AppConfig
        let deps = <UserService as tx_di_core::ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 2);

        // RequestLogger 无依赖
        let deps = <RequestLogger as tx_di_core::ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 0);
    }
    /// 组件描述符构建测试
    #[test]
    fn test_component_descriptor_build() {
        let mut ctx = tx_di_core::BuildContext::new::<PathBuf>(None);

        // 手动调用 build 方法构建组件
        let _db_pool = DbPool::build(&mut ctx);
        // DbPool 是无字段结构体，成功构建即表示正常

        let app_config = AppConfig::build(&mut ctx);
        assert_eq!(app_config.app_name, "tx-di-example");
        assert_eq!(app_config.port, 8080);
    }

    // ── BuildContext API 测试 ──────────────────────────────────────────
    /// BuildContext API 测试
    #[test]
    fn test_build_context_len_and_empty() {
        let ctx = tx_di_core::BuildContext::new::<PathBuf>(None);
        assert_eq!(ctx.len(), 6);
        assert!(!ctx.is_empty());
    }
    /// 构建后
    #[test]
    fn test_build_context_after_initialization() {
        let ctx = create_full_context();
        // 初始化后应该有 6 个组件（包括 AppAllConfig + DbPool, AppConfig, UserService, RequestLogger, AppServer）
        assert_eq!(ctx.len(), 6);
        assert!(!ctx.is_empty());
    }
    /// take 测试
    #[test]
    fn test_take_removes_from_context() {
        let mut ctx = create_full_context();

        // take 之前可以 inject
        let _db_before = ctx.inject::<DbPool>();

        // take 取走所有权
        let _server = ctx.take::<AppServer>();

        // take 之后 AppServer 不再存在于 ctx 中
        // 注意：这里不能再次 inject AppServer，会 panic
        // 但其他组件仍然可用
        let _db_after = ctx.inject::<DbPool>();
        assert!(true, "其他组件仍然可用");
    }

    // ── 边界情况测试 ───────────────────────────────────────────────────
    /// 线程安全测试
    #[test]
    fn test_singleton_thread_safety() {
        use std::thread;

        let mut ctx = create_full_context();
        let db = ctx.inject::<DbPool>();

        // 在多个线程中使用同一个 Arc
        let handles: Vec<_> = (0..5)
            .map(|_| {
                let db_clone = db.clone();
                thread::spawn(move || {
                    // 验证所有线程都持有相同的实例
                    Arc::as_ptr(&db_clone) as usize
                })
            })
            .collect();

        let pointers: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // 所有线程的指针应该相同
        for ptr in &pointers {
            assert_eq!(ptr, &pointers[0], "所有线程应该持有相同的 DbPool 实例");
        }
    }
    /// 原型状态隔离测试
    #[test]
    fn test_prototype_state_isolation() {
        let mut ctx = create_full_context();

        // 创建多个原型实例并修改它们的状态
        let mut loggers = vec![];
        for i in 0..3 {
            let logger = ctx.inject::<RequestLogger>();
            for _ in 0..(i + 1) {
                logger.log(&format!("Message from logger {}", i));
            }
            loggers.push(logger);
        }

        // 验证每个实例的状态独立
        assert_eq!(loggers[0].count(), 1);
        assert_eq!(loggers[1].count(), 2);
        assert_eq!(loggers[2].count(), 3);
    }
    /// 无依赖组件测试
    #[test]
    fn test_component_with_no_dependencies() {
        let mut ctx = create_full_context();

        // DbPool 没有依赖，应该可以直接注入
        let _db = ctx.inject::<DbPool>();
        assert!(true, "无依赖组件应该可以成功注入");
    }
    /// 多个依赖组件测试
    #[test]
    fn test_component_with_multiple_dependencies() {
        let mut ctx = create_full_context();

        // UserService 有两个依赖：DbPool 和 AppConfig
        let user_svc = ctx.inject::<UserService>();

        // 验证两个依赖都被正确注入
        assert!(Arc::ptr_eq(&user_svc.db, &ctx.inject::<DbPool>()));
        assert!(Arc::ptr_eq(&user_svc.config, &ctx.inject::<AppConfig>()));
    }

    // ── 调试功能测试 ───────────────────────────────────────────────────
    /// 调试功能测试
    #[test]
    fn test_debug_registry_output() {
        // 这个测试主要验证 debug_registry 不会 panic
        tx_di_core::BuildContext::debug_registry();
        assert!(true, "debug_registry 应该正常执行");
    }

    // ── 配置文件加载测试 ───────────────────────────────────────────────
    /// 测试从配置文件加载组件
    #[test]
    fn test_load_from_config_file() {
        // 使用示例配置文件
        let ctx = BuildContext::new(Some("../configs/di-config.toml"));
        
        // 应该加载了 5 个组件（包括 AppAllConfig）
        assert_eq!(ctx.len(), 6);
        
        // 验证可以注入组件
        let mut ctx = ctx;
        let _db = ctx.inject::<DbPool>();
        let _config = ctx.inject::<AppConfig>();
        assert_eq!(_config.app_name, "my-app-from-config", "应该从配置文件读取 app_name");
        assert_eq!(_config.port, 9090, "应该从配置文件读取 port");

    }

    /// 测试配置文件中的值被正确加载到 AppConfig
    #[test]
    fn test_config_file_values_loaded_to_app_config() {
        // 从配置文件加载
        let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
        
        // 获取 AppConfig，应该使用配置文件中的值
        let config = ctx.inject::<AppConfig>();
        
        // 验证配置文件中的值被正确加载
        assert_eq!(config.app_name, "my-app-from-config", "应该从配置文件读取 app_name");
        assert_eq!(config.port, 9090, "应该从配置文件读取 port");
    }

    /// 测试全局配置对象 AppAllConfig 可以访问原始 TOML 数据
    #[test]
    fn test_global_config_access_raw_toml() {
        let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
        
        // 注入全局配置对象
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();
        
        // 验证可以从 TOML 中读取嵌套的值
        let app_name: Option<String> = global_config.get("app_config.app_name");
        assert_eq!(app_name, Some("my-app-from-config".to_string()));
        
        let port: Option<u16> = global_config.get("app_config.port");
        assert_eq!(port, Some(9090));
    }

    /// 测试配置文件不存在时使用默认值
    #[test]
    fn test_missing_config_file_uses_defaults() {
        // 使用不存在的配置文件路径
        let mut ctx = BuildContext::new(Some("nonexistent/config.toml"));
        
        // 仍然可以正常注入，使用默认值
        let config = ctx.inject::<AppConfig>();
        
        // 验证使用了默认值（Deserialize::default）
        assert_eq!(config.app_name, "tx-di-example", "配置文件不存在时应使用默认字符串");
        assert_eq!(config.port, 8080, "配置文件不存在时应使用默认值 8080");
    }

    /// 测试 get_or_default 方法
    #[test]
    fn test_config_get_or_default() {
        let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
        
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();
        
        // 存在的键返回配置值
        let app_name = global_config.get_or_default("app_config.app_name", "default-name".to_string());
        assert_eq!(app_name, "my-app-from-config");
        
        // 不存在的键返回默认值
        let missing_key = global_config.get_or_default("nonexistent.key", "fallback".to_string());
        assert_eq!(missing_key, "fallback");
    }

    /// 测试复杂配置文件的多层级访问
    #[test]
    fn test_complex_config_nested_access() {
        let mut ctx = BuildContext::new(Some("../configs/complex-config.toml"));
        
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();
        
        // 验证可以访问多层级配置
        let db_host: Option<String> = global_config.get("database.host");
        assert_eq!(db_host, Some("localhost".to_string()));
        
        let db_port: Option<u16> = global_config.get("database.port");
        assert_eq!(db_port, Some(5432));
        
        let log_level: Option<String> = global_config.get("logging.level");
        assert_eq!(log_level, Some("info".to_string()));
        
        // AppConfig 应该从 complex-config.toml 加载
        let config = ctx.inject::<AppConfig>();
        assert_eq!(config.app_name, "production-app");
        assert_eq!(config.port, 3000);
    }

    /// 测试配置值的类型转换
    #[test]
    fn test_config_value_type_conversion() {
        let mut ctx = BuildContext::new(Some("../configs/complex-config.toml"));
        
        let global_config = ctx.inject::<tx_di_core::AppAllConfig>();
        
        // 测试不同类型的数据提取
        let port_as_u32: Option<u32> = global_config.get("app_config.port");
        assert_eq!(port_as_u32, Some(3000u32));
        
        let port_as_u64: Option<u64> = global_config.get("app_config.port");
        assert_eq!(port_as_u64, Some(3000u64));
        
        // 类型不匹配时返回 None
        let port_as_string: Option<String> = global_config.get("app_config.port");
        assert_eq!(port_as_string, None, "数字类型不能直接转换为 String");
    }

    /// 测试 AppAllConfig 在上下文中是单例
    #[test]
    fn test_app_all_config_is_singleton() {
        let mut ctx = BuildContext::new(Some("../configs/di-config.toml"));
        
        // 多次注入应该返回同一个实例
        let config1 = ctx.inject::<tx_di_core::AppAllConfig>();
        let config2 = ctx.inject::<tx_di_core::AppAllConfig>();
        
        assert!(std::sync::Arc::ptr_eq(&config1, &config2), "AppAllConfig 应该是单例");
    }

    /// 测试自动扫描模式
    #[test]
    fn test_auto_scan_mode() {
        // None 表示自动扫描所有注册的组件
        let ctx = BuildContext::new::<PathBuf>(None);
        
        // 应该加载了 6 个组件（包括 AppAllConfig）
        assert_eq!(ctx.len(), 6);
    }
}
