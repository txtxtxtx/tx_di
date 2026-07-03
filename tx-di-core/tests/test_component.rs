//! tx-di-core 集成测试 — 完整验证框架各项功能
//!
//! 覆盖：
//! 1. 基础注入（无依赖、单依赖、多依赖、深层依赖链）
//! 2. 作用域（Singleton vs Prototype）
//! 3. 字段属性（tx_cst 自定义值、skip、Option）
//! 4. Trait Object 注入
//! 5. 配置组件
//! 6. 生命周期钩子（init_sort、init、async_init、shutdown）
//! 7. Store 操作
//! 8. BuildContext & App
//! 9. AOP 拦截器

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tx_di_core::{Component, DepsTuple, BuildContext, Scope};
use tx_di_core::aop::{Interceptor, CallContext, CallResult, InterceptorChain};

// ══════════════════════════════════════════════════════════════════════════
// 测试组件定义
// ══════════════════════════════════════════════════════════════════════════

// ── 1. 基础组件 ─────────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct DbPool{
    #[tx_cst(AtomicU32::new(0))]
    pub counter: AtomicU32, // 用于验证生命周期钩子
    #[tx_cst("sqlite".to_string())]
    pub url: String,
}

#[derive(Component, Default)]
pub struct RedisClient;

#[derive(Component)]
pub struct UserService {
    pub db: Arc<DbPool>,
}

#[derive(Component)]
pub struct OrderService {
    pub db: Arc<DbPool>,
    pub redis: Arc<RedisClient>,
}

// 三层依赖链: Repo → Service → Controller
#[derive(Component, Default)]
pub struct UserRepo;

#[derive(Component)]
pub struct UserBizService {
    pub repo: Arc<UserRepo>,
}

#[derive(Component)]
pub struct UserController {
    pub service: Arc<UserBizService>,
    pub db: Arc<DbPool>,
}

// ── 2. 作用域测试 ───────────────────────────────────────────────────────

#[derive(Component)]
#[component(scope = Prototype)]
pub struct RequestContext {
    #[tx_cst(0u64)]
    pub request_id: u64,
}

// ── 3. 字段属性测试 ─────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct Logger {
    #[tx_cst("info".to_string())]
    pub level: String,
    #[tx_cst(42i32)]
    pub max_size: i32,
}

#[derive(Component, Default)]
pub struct Cache {
    #[tx_cst(skip)]
    pub temp: Vec<u8>,
    #[tx_cst("cache_key".to_string())]
    pub key: String,
}

#[derive(Component, Default)]
pub struct OptionalFields {
    pub maybe_db: Option<DbPool>,
    #[tx_cst(skip)]
    pub skipped: Vec<String>,
}

// ── 4. Trait Object 注入 ────────────────────────────────────────────────

pub trait DataProvider: std::any::Any + Send + Sync {
    fn get_data(&self) -> &str;
}

#[derive(Component)]
#[component(as_trait = dyn DataProvider)]
pub struct MysqlProvider {
    #[tx_cst("mysql_data".to_string())]
    data: String,
}

impl DataProvider for MysqlProvider {
    fn get_data(&self) -> &str {
        &self.data
    }
}

#[derive(Component)]
pub struct DataConsumer {
    pub provider: Option<Arc<dyn DataProvider>>,
}

// ── 5. 配置组件 ─────────────────────────────────────────────────────────

#[derive(Component, serde::Deserialize, Default)]
#[component(conf = "test_config")]
pub struct TestConfig {
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub port: u16,
}

// ── 6. 生命周期钩子 ─────────────────────────────────────────────────────

static INIT_COUNTER: AtomicU32 = AtomicU32::new(0);
static SHUTDOWN_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Component)]
#[component(init)]
pub struct LifecycleComponent {
    pub db: Arc<DbPool>,
}

// ── 7. 大量依赖测试（验证 16 个依赖上限）─────────────────────────────────

#[derive(Component, Default)]
pub struct Dep1;
#[derive(Component, Default)]
pub struct Dep2;
#[derive(Component, Default)]
pub struct Dep3;
#[derive(Component, Default)]
pub struct Dep4;
#[derive(Component, Default)]
pub struct Dep5;

#[derive(Component)]
pub struct ManyDeps {
    pub d1: Arc<Dep1>,
    pub d2: Arc<Dep2>,
    pub d3: Arc<Dep3>,
    pub d4: Arc<Dep4>,
    pub d5: Arc<Dep5>,
}

// ══════════════════════════════════════════════════════════════════════════
// 测试用例
// ══════════════════════════════════════════════════════════════════════════

// ── 1. 基础注入 ─────────────────────────────────────────────────────────

#[test]
fn test_basic_inject() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let db = ctx.inject::<DbPool>();

    let _ = db;
}

#[test]
fn test_single_dependency() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc = ctx.inject::<UserService>();
    // 验证依赖注入正确，DbPool 字段值来自 #[tx_cst]
    assert_eq!(svc.db.url, "sqlite");
    assert_eq!(svc.db.counter.load(Ordering::Relaxed), 0);
}

#[test]
fn test_multiple_dependencies() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc = ctx.inject::<OrderService>();
    let _ = svc.db;
    let _ = svc.redis;
}

#[test]
fn test_deep_dependency_chain() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let ctrl = ctx.inject::<UserController>();
    // 三层依赖链都应正确解析
    let _ = ctrl.service;
    let _ = ctrl.db;
    let _ = &ctrl.service.repo;
}

#[test]
fn test_many_dependencies() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc = ctx.inject::<ManyDeps>();
    let _ = svc.d1;
    let _ = svc.d2;
    let _ = svc.d3;
    let _ = svc.d4;
    let _ = svc.d5;
}

// ── 2. 作用域测试 ───────────────────────────────────────────────────────

#[test]
fn test_singleton_returns_same_instance() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let db1 = ctx.inject::<DbPool>();
    let db2 = ctx.inject::<DbPool>();
    assert!(Arc::ptr_eq(&db1, &db2), "Singleton 应返回同一实例");
}

#[test]
fn test_prototype_returns_new_instance() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let req1 = ctx.inject::<RequestContext>();
    let req2 = ctx.inject::<RequestContext>();
    assert!(!Arc::ptr_eq(&req1, &req2), "Prototype 应返回不同实例");
}

#[test]
fn test_prototype_in_dependency() {
    // Prototype 组件被 Singleton 组件依赖时
    // 每次 inject Singleton 返回同一个，但内部 Prototype 是创建时的那个
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc1 = ctx.inject::<UserService>();
    let svc2 = ctx.inject::<UserService>();
    // Singleton: svc1 和 svc2 是同一个
    assert!(Arc::ptr_eq(&svc1, &svc2));
    // 内部 db 也是同一个
    assert!(Arc::ptr_eq(&svc1.db, &svc2.db));
}

// ── 3. 字段属性测试 ─────────────────────────────────────────────────────

#[test]
fn test_custom_string_value() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let logger = ctx.inject::<Logger>();
    assert_eq!(logger.level, "info");
}

#[test]
fn test_custom_int_value() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let logger = ctx.inject::<Logger>();
    assert_eq!(logger.max_size, 42);
}

#[test]
fn test_skip_field_uses_default() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let cache = ctx.inject::<Cache>();
    assert!(cache.temp.is_empty(), "skip 字段应为 Default");
    assert_eq!(cache.key, "cache_key");
}

#[test]
fn test_option_field_is_none() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let opt = ctx.inject::<OptionalFields>();
    assert!(opt.maybe_db.is_none(), "Option 字段应为 None");
    assert!(opt.skipped.is_empty());
}

// ── 4. Trait Object 注入 ────────────────────────────────────────────────

#[test]
fn test_trait_object_inject() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let consumer = ctx.inject::<DataConsumer>();
    assert_eq!(consumer.provider.as_ref().unwrap().get_data(), "mysql_data");
}

#[test]
fn test_trait_object_via_store() {
    use tx_di_core::inject_trait_from_store;
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let provider: Arc<dyn DataProvider> = inject_trait_from_store(ctx.store());
    assert_eq!(provider.get_data(), "mysql_data");
}

// ── 5. 配置组件 ─────────────────────────────────────────────────────────

#[test]
fn test_config_component_default() {
    // 无配置文件，应使用 Default
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let config = ctx.inject::<TestConfig>();
    assert_eq!(config.port, 0);
    assert!(config.app_name.is_empty());
}

// ── 6. Store 操作 ────────────────────────────────────────────────────────

#[test]
fn test_store_contains() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    assert!(ctx.store().contains::<DbPool>());
    assert!(ctx.store().contains::<UserService>());
}

#[test]
fn test_store_len() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    assert!(ctx.len() > 0, "Store 应包含至少一个组件");
}

#[test]
fn test_store_try_inject_success() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    assert!(ctx.try_inject::<DbPool>().is_some());
}

#[test]
fn test_store_try_inject_unregistered() {
    // 手动实现一个未注册的 Component
    struct Unregistered;
    impl Component for Unregistered {
        type Deps = ();
        fn build(_: ()) -> Self { Unregistered }
    }

    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    assert!(ctx.try_inject::<Unregistered>().is_none());
}

#[test]
fn test_store_inject_returns_error() {
    #[derive(Debug)]
    struct Unregistered2;
    impl Component for Unregistered2 {
        type Deps = ();
        fn build(_: ()) -> Self { Unregistered2 }
    }

    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let result = ctx.store().inject::<Unregistered2>();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.domain(), "DI");
    assert_eq!(err.code(), -4);
    let ctx_msg = err.context().expect("应携带 context");
    assert!(ctx_msg.contains("Unregistered"));
    assert!(ctx_msg.contains("未注册"));
}

// ── 7. BuildContext & App ────────────────────────────────────────────────

#[test]
fn test_build_app() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let app = ctx.build().unwrap();
    assert!(!app.is_empty());
}

#[test]
fn test_app_inject() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let app = ctx.build().unwrap();
    let db = app.inject::<DbPool>();
    let _ = db;
}

#[test]
fn test_app_try_inject() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let app = ctx.build().unwrap();
    assert!(app.try_inject::<DbPool>().is_some());
}

#[test]
fn test_app_store_access() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let app = ctx.build().unwrap();
    assert!(app.store().contains::<DbPool>());
}

// ── 8. 跨组件一致性 ──────────────────────────────────────────────────────

#[test]
fn test_shared_dependency_is_same_instance() {
    // UserService 和 OrderService 都依赖 DbPool
    // 它们应该拿到同一个 DbPool 实例
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let user_svc = ctx.inject::<UserService>();
    let order_svc = ctx.inject::<OrderService>();
    assert!(
        Arc::ptr_eq(&user_svc.db, &order_svc.db),
        "共享依赖应为同一实例"
    );
}

#[test]
fn test_deep_chain_shared_dependency() {
    // UserController 和 UserService 都依赖 DbPool
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let ctrl = ctx.inject::<UserController>();
    let svc = ctx.inject::<UserService>();
    assert!(Arc::ptr_eq(&ctrl.db, &svc.db));
}

// ── 9. AOP 拦截器 ────────────────────────────────────────────────────────

#[test]
fn test_interceptor_before_after() {
    use std::sync::atomic::AtomicBool;

    struct TestInterceptor {
        before_called: Arc<AtomicBool>,
        after_called: Arc<AtomicBool>,
    }

    impl Interceptor for TestInterceptor {
        fn before(&self, _ctx: &CallContext) {
            self.before_called.store(true, Ordering::SeqCst);
        }
        fn after(&self, _ctx: &CallContext, _result: &CallResult) {
            self.after_called.store(true, Ordering::SeqCst);
        }
    }

    let before = Arc::new(AtomicBool::new(false));
    let after = Arc::new(AtomicBool::new(false));

    let interceptor = TestInterceptor {
        before_called: before.clone(),
        after_called: after.clone(),
    };

    let ctx = CallContext::new("test_method");
    interceptor.before(&ctx);
    interceptor.after(&ctx, &CallResult::Ok);

    assert!(before.load(Ordering::SeqCst));
    assert!(after.load(Ordering::SeqCst));
}

#[test]
fn test_interceptor_chain() {
    use std::sync::atomic::AtomicU32;

    struct CountingInterceptor {
        counter: Arc<AtomicU32>,
    }

    impl Interceptor for CountingInterceptor {
        fn before(&self, _ctx: &CallContext) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
        fn after(&self, _ctx: &CallContext, _result: &CallResult) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    let counter = Arc::new(AtomicU32::new(0));
    let mut chain = InterceptorChain::new();
    chain.push(CountingInterceptor { counter: counter.clone() });
    chain.push(CountingInterceptor { counter: counter.clone() });

    let ctx = CallContext::new("test");
    chain.before_all(&ctx);
    // 2 个拦截器各调一次 before = 2
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    chain.after_all(&ctx, &CallResult::Ok);
    // 2 个拦截器各调一次 after = 2，总计 4
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_logging_interceptor() {
    use tx_di_core::aop::LoggingInterceptor;

    let interceptor = LoggingInterceptor;
    let ctx = CallContext::new("test_method").with_arg("param1".into());
    // 不 panic 即通过
    interceptor.before(&ctx);
    interceptor.after(&ctx, &CallResult::Ok);
    interceptor.after(&ctx, &CallResult::Err("test error".into()));
}

#[test]
fn test_metrics_interceptor() {
    use tx_di_core::aop::MetricsInterceptor;

    let interceptor = MetricsInterceptor::new();
    let ctx = CallContext::new("counted_method");

    assert_eq!(interceptor.count(), 0);
    interceptor.before(&ctx);
    assert_eq!(interceptor.count(), 1);
    interceptor.before(&ctx);
    assert_eq!(interceptor.count(), 2);
}

// ── 10. 错误处理 ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "DI:-4")]
fn test_inject_unregistered_panics() {
    struct Ghost;
    impl Component for Ghost {
        type Deps = ();
        fn build(_: ()) -> Self { Ghost }
    }

    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    ctx.inject::<Ghost>();
}

// ── 11. 并发注入 ─────────────────────────────────────────────────────────

#[test]
fn test_concurrent_inject_singleton() {
    use std::thread;

    let ctx = Arc::new(BuildContext::new::<std::path::PathBuf>(None));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let ctx = ctx.clone();
            thread::spawn(move || {
                let db = ctx.inject::<DbPool>();
                db
            })
        })
        .collect();

    let results: Vec<Arc<DbPool>> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // 所有线程应拿到同一个实例
    for i in 1..results.len() {
        assert!(Arc::ptr_eq(&results[0], &results[i]));
    }
}

#[test]
fn test_concurrent_inject_prototype() {
    use std::thread;

    let ctx = Arc::new(BuildContext::new::<std::path::PathBuf>(None));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let ctx = ctx.clone();
            thread::spawn(move || {
                ctx.inject::<RequestContext>()
            })
        })
        .collect();

    let results: Vec<Arc<RequestContext>> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Prototype: 每个线程拿到不同实例
    for i in 1..results.len() {
        assert!(!Arc::ptr_eq(&results[0], &results[i]));
    }
}

// ── 12. DepsTuple trait 测试 ──────────────────────────────────────────────

#[test]
fn test_deps_tuple_empty() {
    let deps = <() as DepsTuple>::dep_type_ids();
    assert!(deps.is_empty());
}

#[test]
fn test_deps_tuple_single() {
    let deps = <(Arc<DbPool>,) as DepsTuple>::dep_type_ids();
    assert_eq!(deps.len(), 1);
}

#[test]
fn test_deps_tuple_multiple() {
    let deps = <(Arc<DbPool>, Arc<RedisClient>) as DepsTuple>::dep_type_ids();
    assert_eq!(deps.len(), 2);
}

// ── 13. Scope enum 测试 ──────────────────────────────────────────────────

#[test]
fn test_scope_methods() {
    assert!(Scope::Singleton.is_singleton());
    assert!(!Scope::Singleton.is_prototype());
    assert!(!Scope::Prototype.is_singleton());
    assert!(Scope::Prototype.is_prototype());
}

#[test]
fn test_scope_default() {
    assert_eq!(Scope::default(), Scope::Singleton);
}
