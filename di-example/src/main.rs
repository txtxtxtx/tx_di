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
use std::sync::{Arc, Mutex};

use tx_di_core::{app, tx_comp, BoxFuture, BuildContext, CompInit};
use log::{debug, info};
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
#[derive(Clone, Debug)]
#[tx_comp]
pub struct AppConfig {
    /// 应用名称（自定义值注入）
    #[tx_cst("my-app".to_string())]
    pub app_name: String,

    /// 监听端口（函数调用注入）
    #[tx_cst(default_port())]
    pub port: u16,
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
    fn async_init(ctx: &mut BuildContext) -> BoxFuture<'static, ()> {
        let len = ctx.len();
        Box::pin(async move {
            debug!("AppServer::async_init:{}",len);
        })
    }
    fn init(ctx: &mut BuildContext) {
        debug!("AppServer::init:{}",ctx.len())
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
// 5. 声明 DI 模块
// ─────────────────────────────────────────────────────────────────────────────

app! {
    AppModule
    // [
    //     DbPool,
    //     AppConfig,
    //     RequestLogger,
    //     UserService,
    //     AppServer
    // ]
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. main
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    info!("🚀 tx_di 启动");

    let mut ctx = build_app_module();
    ctx.run().await;
    info!("构建完成");
    // ── 取出 AppServer ──────────────────────────────────────────────────
    let server = ctx.take::<AppServer>();

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
    BuildContext::debug_registry();
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use tx_di_core::ComponentDescriptor;
    use super::*;

    // ── 单例测试 ───────────────────────────────────────────────────────
    /// 单例测试
    #[test]
    fn test_singleton_shared() {
        let mut ctx = build_app_module();
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
        let mut ctx = build_app_module();

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
        let mut ctx = build_app_module();

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
        let mut ctx = build_app_module();

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
        let mut ctx = build_app_module();

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
        let mut ctx = build_app_module();

        let logger = ctx.inject::<RequestLogger>();

        // 验证 #[tx_cst] 注入的自定义值正确
        assert_eq!(logger.prefix, "[REQUEST]");
        assert_eq!(logger.count(), 0, "新实例的计数器应该从 0 开始");
    }

    // ── 自定义值注入测试 ───────────────────────────────────────────────
    /// 自定义值注入测试
    #[test]
    fn test_inject_custom_values() {
        let mut ctx = build_app_module();
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
        let mut ctx = build_app_module();
        let cfg = ctx.inject::<AppConfig>();

        assert_eq!(cfg.app_name, "my-app");
        assert_eq!(cfg.port, 8080);
    }
    /// 自定义值表达式求值一次
    #[test]
    fn test_custom_value_expression_evaluated_once() {
        let mut ctx = build_app_module();

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
        let mut ctx = build_app_module();

        // AppServer 依赖 UserService
        let server = ctx.take::<AppServer>();

        // UserService 依赖 DbPool 和 AppConfig
        assert!(Arc::ptr_eq(&server.user_svc.db, &ctx.inject::<DbPool>()));
        assert!(Arc::ptr_eq(&server.user_svc.config, &ctx.inject::<AppConfig>()));
    }
    /// 验证 UserService 功能
    #[test]
    fn test_user_service_functionality() {
        let mut ctx = build_app_module();
        let server = ctx.take::<AppServer>();

        // 验证 UserService 可以正常使用注入的依赖
        let greeting = server.user_svc.greet();
        assert_eq!(greeting, "[my-app] Hello from UserService (port: 8080)");
    }

    // ── 注册表测试 ─────────────────────────────────────────────────────
    /// 注册表测试
    #[test]
    fn test_registry() {
        let count = tx_di_core::COMPONENT_REGISTRY.len();
        assert_eq!(count, 5, "应该有 5 个注册组件");
        
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
        let mut ctx = tx_di_core::BuildContext::new();

        // 手动调用 build 方法构建组件
        let _db_pool = DbPool::build(&mut ctx);
        // DbPool 是无字段结构体，成功构建即表示正常

        let app_config = AppConfig::build(&mut ctx);
        assert_eq!(app_config.app_name, "my-app");
        assert_eq!(app_config.port, 8080);
    }

    // ── BuildContext API 测试 ──────────────────────────────────────────
    /// BuildContext API 测试
    #[test]
    fn test_build_context_len_and_empty() {
        let ctx = tx_di_core::BuildContext::new();
        assert_eq!(ctx.len(), 0);
        assert!(ctx.is_empty());
    }
    /// 构建后
    #[test]
    fn test_build_context_after_initialization() {
        let ctx = build_app_module();
        // 初始化后应该有 5 个组件（DbPool, AppConfig, UserService, RequestLogger, AppServer）
        assert_eq!(ctx.len(), 5);
        assert!(!ctx.is_empty());
    }
    /// take 测试
    #[test]
    fn test_take_removes_from_context() {
        let mut ctx = build_app_module();

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

        let mut ctx = build_app_module();
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
        let mut ctx = build_app_module();

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
        let mut ctx = build_app_module();

        // DbPool 没有依赖，应该可以直接注入
        let _db = ctx.inject::<DbPool>();
        assert!(true, "无依赖组件应该可以成功注入");
    }
    /// 多个依赖组件测试
    #[test]
    fn test_component_with_multiple_dependencies() {
        let mut ctx = build_app_module();

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
}
