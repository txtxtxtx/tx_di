//! # di-example
//!
//! 演示 di-framework 的全部特性：
//!
//! ## 设计原则
//!
//! **scope 标记在被注入者上**，消费者字段写裸类型即可：
//!
//! ```text
//! #[component]                        // DbPool 自己声明为单例
//! pub struct DbPool { ... }
//!
//! #[component(scope = Prototype)]     // RequestLogger 自己声明为原型
//! pub struct RequestLogger { ... }
//!
//! #[component]                        // AppServer 不需要知道依赖的 scope
//! pub struct AppServer {
//!     pub db:     Arc<DbPool>,        // 框架自动注入共享 Arc
//!     pub logger: Arc<RequestLogger>, // 框架自动注入新实例
//! }
//! ```
//!
//! ## 依赖关系
//!
//! ```text
//!  ┌──────────┐   ┌────────────┐
//!  │  DbPool  │   │ AppConfig  │  ← #[component]（默认 Singleton）
//!  └────┬─────┘   └─────┬──────┘
//!       │                    │
//!       ▼ Singleton          ▼ Singleton (自动)
//!  ┌──────────────────┐  ┌──────────────────────┐
//!  │    UserService    │  │    RequestLogger     │← #[component(scope = Prototype)]
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
//!              │  extra       │ ← #[inject(expr)]
//!              └──────────────┘
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use di_macros::{app, component};

// ─────────────────────────────────────────────────────────────────────────────
// 1. 无依赖的单例组件
// ─────────────────────────────────────────────────────────────────────────────

/// 数据库连接池（单例，全局共享）
#[derive(Clone, Debug)]
#[component]
pub struct DbPool {
    // 实际项目中这里是 sea_orm::DatabaseConnection
    // 无字段的组件：build() → Self {}
}

/// 应用配置（单例，通过 #[inject] 注入自定义值）
#[derive(Clone, Debug)]
#[component]
pub struct AppConfig {
    /// 应用名称（自定义值注入）
    #[inject("my-app".to_string())]
    pub app_name: String,

    /// 监听端口（函数调用注入）
    #[inject(default_port())]
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
#[component(scope = di_core::Prototype)]
pub struct RequestLogger {
    /// 日志前缀
    #[inject("[REQUEST]".to_string())]
    pub prefix: String,

    /// 请求计数器（每个实例独立，通过 #[inject] 初始化）
    #[inject(Arc::new(Mutex::new(0u64)))]
    count: Arc<Mutex<u64>>,
}

impl RequestLogger {
    pub fn log(&self, msg: &str) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        println!("{} [#{}] {}", self.prefix, *c, msg);
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
#[component]
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
#[component]
pub struct AppServer {
    /// 共享的 UserService
    pub user_svc: Arc<UserService>,

    /// 原型注入：AppServer 独占自己的 RequestLogger 实例
    /// （每次 inject() 对 Prototype 组件调用工厂，构造新实例）
    pub logger: Arc<RequestLogger>,

    /// 自定义值注入：HashMap 不通过 ctx，直接用表达式构造
    #[inject(default_headers())]
    pub default_headers: HashMap<String, String>,

    /// 自定义值注入
    #[inject("0.0.0.0:8080".to_string())]
    pub bind_addr: String,
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
    AppModule [
        // 无依赖的先放
        DbPool,
        AppConfig,
        // 原型组件（无 ctx 依赖）
        RequestLogger,
        // 依赖上面组件
        UserService,
        // 最终聚合层
        AppServer,
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== di-framework v3 演示（scope 在被注入者上）===\n");

    let mut ctx = build_app_module();

    // ── 取出 AppServer ──────────────────────────────────────────────────
    let server = ctx.take::<AppServer>();

    println!("✅ AppServer 构建完成");
    println!("   bind_addr  = {}", server.bind_addr);
    println!("   headers    = {:?}", server.default_headers);

    // ── 验证单例共享 ────────────────────────────────────────────────────
    let greeting = server.user_svc.greet();
    println!("\n📢 UserService.greet() = {}", greeting);

    // server.user_svc.db 是 Arc<DbPool>，Arc derefs → 可以直接比较指针
    let db_in_ctx = ctx.inject::<DbPool>();
    let db_in_ctx_ptr = Arc::as_ptr(&db_in_ctx);
    println!(
       "   DbPool Arc 共享 = {}（{} vs {}）",
       if Arc::as_ptr(&server.user_svc.db) == db_in_ctx_ptr { "✅ 同一家" } else { "❌ 不同实例" },
       Arc::as_ptr(&server.user_svc.db) as usize, db_in_ctx_ptr as usize
    );

    // ── 验证原型独立性 ─────────────────────────────────────────────────
    println!("\n🔄 验证 Prototype 独立性：");
    server.logger.log("第一条日志");
    server.logger.log("第二条日志");

    // 从 ctx 再次 inject Prototype 组件 → 构造新实例
    let another_logger = ctx.inject::<RequestLogger>();
    another_logger.log("另一个 logger 的第一条");
    println!(
        "   server.logger.count = {}（独立）",
        server.logger.count()
    );
    println!(
        "   another_logger.count = {}（独立，从 0 开始）",
        another_logger.count()
    );

    // ── 验证 AppConfig ─────────────────────────────────────────────────
    println!("\n⚙️  AppConfig（#[inject] 自定义值）：");
    let cfg_arc = ctx.inject::<AppConfig>();
    println!("   app_name = {}", cfg_arc.app_name);
    println!("   port     = {}", cfg_arc.port);
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
            Arc::as_ptr(&l1), Arc::as_ptr(&l2),
            "Prototype 实例应该相互独立（不同 Arc 指针）"
        );
        assert_eq!(l1.count(), 1);
        assert_eq!(l2.count(), 1, "每个 logger 计数器独立");
    }

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

    #[test]
    fn test_app_config_inject() {
        let mut ctx = build_app_module();
        let cfg = ctx.inject::<AppConfig>();

        assert_eq!(cfg.app_name, "my-app");
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn test_registry() {
        let count = di_core::COMPONENT_REGISTRY.len();
        println!("注册组件数：{}", count);
        for meta in di_core::COMPONENT_REGISTRY.iter() {
            println!("  {:20} scope={:?}  deps={}", meta.name, meta.scope, meta.deps.len());
        }
        assert!(count >= 5);
    }

    #[test]
    fn test_scope_on_component() {
        // 验证 scope 确实在组件自己身上
        assert_eq!(di_core::Scope::Singleton, <DbPool as di_core::ComponentDescriptor>::SCOPE);
        assert_eq!(di_core::Scope::Singleton, <AppConfig as di_core::ComponentDescriptor>::SCOPE);
        assert_eq!(di_core::Scope::Singleton, <UserService as di_core::ComponentDescriptor>::SCOPE);
        assert_eq!(di_core::Scope::Singleton, <AppServer as di_core::ComponentDescriptor>::SCOPE);
        assert_eq!(di_core::Scope::Prototype, <RequestLogger as di_core::ComponentDescriptor>::SCOPE);

        // UserService 依赖 DbPool 和 AppConfig
        let deps = <UserService as di_core::ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 2);

        // RequestLogger 无依赖
        let deps = <RequestLogger as di_core::ComponentDescriptor>::DEP_IDS;
        assert_eq!(deps.len(), 0);
    }
}
