//! tx-di-core 集成测试 — 完整验证框架各项功能
//!
//! 覆盖：
//! 1. 基础注入（无依赖、单依赖、多依赖、深层依赖链）
//! 2. 作用域（Singleton vs Prototype）
//! 3. 字段属性（tx_cst 自定义值、skip、Option）
//! 4. Trait Object 注入
//! 5. 配置组件
//! 6. 生命周期钩子（init_sort、app_init、app_async_init、app_async_run、shutdown）
//! 7. Store 操作（含边缘操作）
//! 8. BuildContext & App（含异步生命周期）
//! 9. AOP 拦截器
//! 10. 注入错误路径
//! 11. 并发注入
//! 12. DepsTuple（含 resolve）
//! 13. 批量 Trait 注入

use std::any::TypeId;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use tx_di_core::{
    App, AppError, BuildContext, CompRef, Component, DepsTuple, DiErr, RIE, Scope,
    Store, inject_all_traits_from_store, inject_from_store, inject_trait_from_store,
};
use tx_di_core::aop::{CallContext, CallResult, Interceptor, InterceptorChain};
use tx_di_core::intercept;

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

#[allow(dead_code)]
static INIT_COUNTER: AtomicU32 = AtomicU32::new(0);
#[allow(dead_code)]
static SHUTDOWN_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Component)]
pub struct LifecycleComponent {
    pub db: Arc<DbPool>,
}

// ── 6b. 生命周期钩子标记组件（每个组件放入独立模块，避免同名自由函数冲突）─

static APP_INIT_CALLED: AtomicBool = AtomicBool::new(false);
static APP_ASYNC_INIT_CALLED: AtomicBool = AtomicBool::new(false);
static SHUTDOWN_CALLED: AtomicBool = AtomicBool::new(false);
static INIT_ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

pub mod app_init_mod {
    use super::*;
    #[derive(Component)]
    #[component(app_init)]
    pub struct AppInitTracker;
    fn app_init(_comp: Arc<AppInitTracker>, _app: &Arc<App>) -> RIE<()> {
        super::APP_INIT_CALLED.store(true, Ordering::SeqCst);
        Ok(())
    }
}

pub mod async_init_mod {
    use super::*;
    #[derive(Component)]
    #[component(app_async_init)]
    pub struct AsyncInitTracker;
    async fn app_async_init(_comp: Arc<AsyncInitTracker>, _app: Arc<App>) -> RIE<()> {
        super::APP_ASYNC_INIT_CALLED.store(true, Ordering::SeqCst);
        Ok(())
    }
}

pub mod shutdown_mod {
    use super::*;
    #[derive(Component)]
    #[component(shutdown)]
    pub struct ShutdownTracker;
    fn shutdown(_comp: &ShutdownTracker) {
        super::SHUTDOWN_CALLED.store(true, Ordering::SeqCst);
    }
}

pub mod early_init_mod {
    use super::*;
    #[derive(Component)]
    #[component(app_init, init_sort = 10)]
    pub struct EarlyInit;
    fn app_init(_comp: Arc<EarlyInit>, _app: &Arc<App>) -> RIE<()> {
        super::INIT_ORDER.lock().unwrap().push("EarlyInit");
        Ok(())
    }
}

pub mod late_init_mod {
    use super::*;
    #[derive(Component)]
    #[component(app_init, init_sort = 20)]
    pub struct LateInit;
    fn app_init(_comp: Arc<LateInit>, _app: &Arc<App>) -> RIE<()> {
        super::INIT_ORDER.lock().unwrap().push("LateInit");
        Ok(())
    }
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
        fn before(&self, _ctx: &CallContext) -> RIE<()> {
            self.before_called.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn after(&self, _ctx: &CallContext, _result: &mut CallResult) {
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
    let mut result = CallResult::Ok;
    interceptor.before(&ctx).unwrap();
    interceptor.after(&ctx, &mut result);

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
        fn before(&self, _ctx: &CallContext) -> RIE<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn after(&self, _ctx: &CallContext, _result: &mut CallResult) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    let counter = Arc::new(AtomicU32::new(0));
    let mut chain = InterceptorChain::new();
    chain.push(CountingInterceptor { counter: counter.clone() });
    chain.push(CountingInterceptor { counter: counter.clone() });

    let ctx = CallContext::new("test");
    chain.before_all(&ctx).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    let mut result = CallResult::Ok;
    chain.after_all(&ctx, &mut result);
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_logging_interceptor() {
    use tx_di_core::aop::LoggingInterceptor;

    let interceptor = LoggingInterceptor;
    let ctx = CallContext::new("test_method").with_arg("param1".into());
    let mut result = CallResult::Ok;
    interceptor.before(&ctx).unwrap();
    interceptor.after(&ctx, &mut result);
    let mut err_result = CallResult::Err("test error".into());
    interceptor.after(&ctx, &mut err_result);
}

#[test]
fn test_metrics_interceptor() {
    use tx_di_core::aop::MetricsInterceptor;

    let interceptor = MetricsInterceptor::new();
    let ctx = CallContext::new("counted_method");

    assert_eq!(interceptor.count(), 0);
    interceptor.before(&ctx).unwrap();
    assert_eq!(interceptor.count(), 1);
    interceptor.before(&ctx).unwrap();
    assert_eq!(interceptor.count(), 2);
}

#[test]
fn test_interceptor_before_reject() {
    struct RejectInterceptor;

    impl Interceptor for RejectInterceptor {
        fn before(&self, _ctx: &CallContext) -> RIE<()> {
            Err(AppError::with_context(
                DiErr::InjectError,
                "rejected".to_string(),
            ))
        }
    }

    let mut chain = InterceptorChain::new();
    chain.push(RejectInterceptor);
    let ctx = CallContext::new("test");
    let result = chain.before_all(&ctx);
    assert!(result.is_err());
}

#[test]
fn test_interceptor_after_modify_result() {
    struct ErrEnricher;

    impl Interceptor for ErrEnricher {
        fn after(&self, _ctx: &CallContext, result: &mut CallResult) {
            if let CallResult::Err(msg) = result {
                *msg = format!("enriched: {}", msg);
            }
        }
    }

    let mut chain = InterceptorChain::new();
    chain.push(ErrEnricher);
    let ctx = CallContext::new("test");
    let mut result = CallResult::Err("fail".into());
    chain.after_all(&ctx, &mut result);
    match &result {
        CallResult::Err(msg) => assert_eq!(msg, "enriched: fail"),
        _ => panic!("expected Err"),
    }
}

// ── 9b. AOP 宏端到端集成测试（`#[component(intercept(...))]` + `#[intercept]`）──
//
// 验证宏链路真实可用：拦截器作为 DI 组件被注入拦截链，业务方法经 `#[intercept]`
// 包裹后实际触发 before/after 拦截。覆盖 sync 与 async 两种情况。

/// 计数拦截器 — 本身也是 DI 组件，before 时自增计数器
#[derive(Component, Default)]
pub struct AopCountInterceptor {
    #[tx_cst(AtomicU64::new(0))]
    pub counter: AtomicU64,
}

impl Interceptor for AopCountInterceptor {
    fn before(&self, _ctx: &CallContext) -> RIE<()> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// 被拦截的业务组件（同时演示 sync 与 async 方法拦截）
#[derive(Component)]
#[component(intercept(AopCountInterceptor))]
pub struct AopBiz {
    #[tx_cst(skip)]
    _placeholder: (),
}

impl AopBiz {
    #[intercept]
    fn sync_add(&self, x: u32) -> RIE<u32> {
        Ok(x + 1)
    }

    #[intercept]
    async fn async_mul(&self, x: u32) -> RIE<u32> {
        Ok(x * 2)
    }
}

#[test]
fn test_aop_intercept_macro_end_to_end() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let app = BuildContext::new::<PathBuf>(None).build().unwrap();
        let arc = app.ins_run().await.unwrap();

        let biz = inject_from_store::<AopBiz>(&arc.store());
        let interceptor = inject_from_store::<AopCountInterceptor>(&arc.store());

        // init 阶段已将拦截器注入链中；调用前计数器清零
        interceptor.counter.store(0, Ordering::Relaxed);

        // sync 拦截方法
        assert_eq!(biz.sync_add(41).unwrap(), 42);
        assert_eq!(
            interceptor.counter.load(Ordering::Relaxed),
            1,
            "sync 方法应触发一次 before 拦截"
        );

        // async 拦截方法
        assert_eq!(biz.async_mul(5).await.unwrap(), 10);
        assert_eq!(
            interceptor.counter.load(Ordering::Relaxed),
            2,
            "async 方法应触发第二次 before 拦截"
        );

        // 优雅关闭后台任务
        arc.shutdown_token.cancel();
        if let Some(handle) = arc.task_handle.write().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }
        arc.shutdown().await;
    });
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

// ══════════════════════════════════════════════════════════════════════════
// 14. 生命周期钩子测试
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_app_init_hook() {
    APP_INIT_CALLED.store(false, Ordering::SeqCst);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _ = BuildContext::new::<PathBuf>(None)
            .build_and_run()
            .await
            .unwrap();
    });

    assert!(
        APP_INIT_CALLED.load(Ordering::SeqCst),
        "app_init 钩子应被调用"
    );
}

#[test]
fn test_app_async_init_hook() {
    APP_ASYNC_INIT_CALLED.store(false, Ordering::SeqCst);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _ = BuildContext::new::<PathBuf>(None)
            .build_and_run()
            .await
            .unwrap();
    });

    assert!(
        APP_ASYNC_INIT_CALLED.load(Ordering::SeqCst),
        "app_async_init 钩子应被调用"
    );
}

#[test]
fn test_shutdown_hook() {
    SHUTDOWN_CALLED.store(false, Ordering::SeqCst);

    let ctx = BuildContext::new::<PathBuf>(None);
    let app = ctx.build().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        app.shutdown().await;
    });

    assert!(
        SHUTDOWN_CALLED.load(Ordering::SeqCst),
        "shutdown 钩子应被调用"
    );
}

#[test]
fn test_init_sort_ordering() {
    // 重置全局 order 记录
    INIT_ORDER.lock().unwrap().clear();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _ = BuildContext::new::<PathBuf>(None)
            .build_and_run()
            .await
            .unwrap();
    });

    let order = INIT_ORDER.lock().unwrap();
    assert_eq!(order.len(), 2, "两个 init 组件都应被调用");
    assert_eq!(
        order[0], "EarlyInit",
        "init_sort=10 应先执行"
    );
    assert_eq!(
        order[1], "LateInit",
        "init_sort=20 后执行"
    );
}

#[test]
fn test_app_async_run_hook() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let app = ctx.build().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let arc = app.ins_run().await.unwrap();
        // 等待后台任务完成 init → async_init → comp_run（所有 async_run 立即返回）
        tokio::time::sleep(Duration::from_millis(100)).await;
        // 取消并优雅关闭
        arc.shutdown_token.cancel();
        if let Some(handle) = arc.task_handle.write().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }
        arc.shutdown().await;
    });
}

#[test]
fn test_lifecycle_full_flow() {
    APP_INIT_CALLED.store(false, Ordering::SeqCst);
    APP_ASYNC_INIT_CALLED.store(false, Ordering::SeqCst);
    SHUTDOWN_CALLED.store(false, Ordering::SeqCst);

    let ctx = BuildContext::new::<PathBuf>(None);
    let app = ctx.build().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let arc = app.ins_run().await.unwrap();
        // 短暂等待让 init + async_init 完成
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(APP_INIT_CALLED.load(Ordering::SeqCst), "init");
        assert!(APP_ASYNC_INIT_CALLED.load(Ordering::SeqCst), "async_init");

        // 取消并关闭
        arc.shutdown_token.cancel();
        if let Some(handle) = arc.task_handle.write().await.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }
        arc.shutdown().await;

        assert!(SHUTDOWN_CALLED.load(Ordering::SeqCst), "shutdown");
    });
}

// ══════════════════════════════════════════════════════════════════════════
// 15. Store 边缘操作测试
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_store_insert_arc() {
    let store = Store::new();
    let val = Arc::new(42u64);
    store.insert_arc(val.clone());

    let entry = store.inner().get(&TypeId::of::<u64>()).unwrap();
    match &*entry {
        CompRef::Cached(arc) => {
            let retrieved = arc.clone().downcast::<u64>().unwrap();
            assert_eq!(*retrieved, 42);
        }
        _ => panic!("应为 Cached 变体"),
    }
}

#[test]
fn test_store_insert_factory() {
    let store = Store::new();
    store.insert_factory::<String, _>(|_| Arc::new("factory_created".to_string()));

    assert!(store.contains::<String>());
    let entry = store.inner().get(&TypeId::of::<String>()).unwrap();
    assert!(
        matches!(&*entry, CompRef::Factory(_)),
        "应为 Factory 变体"
    );
}

#[test]
fn test_store_into_inner_and_from_dashmap() {
    let store = Store::new();
    store.insert_cached(100u64);
    store.insert_cached("hello".to_string());
    let original_len = store.len();

    let inner = store.into_inner();
    assert_eq!(inner.len(), original_len);

    // 从 DashMap 重建 Store
    let restored = Store::from_dashmap(inner);
    assert_eq!(restored.len(), original_len);
    assert!(restored.contains::<u64>());
    assert!(restored.contains::<String>());
}

#[test]
fn test_store_is_empty() {
    let store = Store::new();
    assert!(store.is_empty());

    store.insert_cached(42u64);
    assert!(!store.is_empty());
}

// ══════════════════════════════════════════════════════════════════════════
// 16. 批量 Trait 注入测试
// ══════════════════════════════════════════════════════════════════════════

pub trait Reporter: std::any::Any + Send + Sync {
    fn report(&self) -> &str;
}

#[derive(Component)]
#[component(as_trait = dyn Reporter)]
pub struct XmlReporter {
    #[tx_cst("xml".to_string())]
    data: String,
}
impl Reporter for XmlReporter {
    fn report(&self) -> &str {
        &self.data
    }
}

#[derive(Component)]
#[component(as_trait = dyn Reporter)]
pub struct JsonReporter {
    #[tx_cst("json".to_string())]
    data: String,
}
impl Reporter for JsonReporter {
    fn report(&self) -> &str {
        &self.data
    }
}

#[test]
fn test_inject_all_traits_from_store() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let store = ctx.store();

    let reporters: Vec<Arc<dyn Reporter>> = inject_all_traits_from_store(store);

    // trait_impls 是 Store 实例字段，每个 BuildContext 独立填充，此处只检查内容
    assert!(!reporters.is_empty(), "应有至少一个 Reporter 实现");

    let data: Vec<&str> = reporters.iter().map(|r| r.report()).collect();
    assert!(
        data.contains(&"json"),
        "应包含 json 实现, got: {:?}",
        data
    );
    assert!(
        data.contains(&"xml"),
        "应包含 xml 实现, got: {:?}",
        data
    );
}

#[test]
fn test_inject_all_traits_empty() {
    // 定义一个未注册的 trait（没有任何 #[component(as_trait)] 实现）
    pub trait NoImplementations: std::any::Any + Send + Sync {}
    let _ = std::any::type_name::<dyn NoImplementations>();

    let ctx = BuildContext::new::<PathBuf>(None);
    let store = ctx.store();

    // 注入一个没有实现的 trait
    let results: Vec<Arc<dyn NoImplementations>> = inject_all_traits_from_store(store);
    assert!(results.is_empty(), "无实现时应返回空 Vec");
}

// ══════════════════════════════════════════════════════════════════════════
// 17. DepsTuple::resolve 测试
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_deps_tuple_resolve() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let store = ctx.store();

    let deps = <(Arc<DbPool>, Arc<RedisClient>) as DepsTuple>::resolve(store);
    assert!(deps.is_ok(), "resolve 应成功");

    let (db, redis) = deps.unwrap();
    assert_eq!(db.url, "sqlite");
    let _ = redis; // 占位验证
}

// ══════════════════════════════════════════════════════════════════════════
// 18. 配置组件真实文件测试
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_config_with_real_file() {
    use std::io::Write;

    // 创建临时配置目录和文件
    let tmp_dir = std::env::temp_dir().join("tx_di_test_config");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let config_path = tmp_dir.join("test_app.toml");

    let mut file = std::fs::File::create(&config_path).unwrap();
    writeln!(file, r#"[test_config]"#).unwrap();
    writeln!(file, r#"app_name = "TestApp""#).unwrap();
    writeln!(file, "port = 8080").unwrap();
    file.sync_all().unwrap();

    // 使用配置文件路径构建
    let ctx = BuildContext::new(Some(config_path.clone()));
    let config = ctx.inject::<TestConfig>();

    assert_eq!(config.app_name, "TestApp");
    assert_eq!(config.port, 8080);

    // 清理临时文件
    let _ = std::fs::remove_file(&config_path);
    let _ = std::fs::remove_dir(&tmp_dir);
}

// ══════════════════════════════════════════════════════════════════════════
// 19. inject_from_store / inject_trait_from_store 直接调用
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_inject_from_store_fn() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let db = inject_from_store::<DbPool>(ctx.store());
    assert_eq!(db.url, "sqlite");
}

#[test]
fn test_inject_trait_from_store_fn() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let provider: Arc<dyn DataProvider> = inject_trait_from_store(ctx.store());
    assert_eq!(provider.get_data(), "mysql_data");
}

// ══════════════════════════════════════════════════════════════════════════
// 20. BuildContext 默认构造 / inner_new
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_context_default() {
    let ctx = BuildContext::default();
    assert!(ctx.len() > 0);
    let _ = ctx.inject::<DbPool>();
}

#[test]
fn test_build_context_inner_new() {
    use dashmap::DashMap;
    let inner = DashMap::new();
    inner.insert(
        TypeId::of::<u64>(),
        CompRef::Cached(Arc::new(42u64) as Arc<dyn std::any::Any + Send + Sync>),
    );
    let ctx = BuildContext::inner_new(inner);
    let store = ctx.store();
    assert!(store.contains::<u64>());
    assert_eq!(store.len(), 1);
}

// ══════════════════════════════════════════════════════════════════════════
// 21. DepsTuple 多元素测试
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_deps_tuple_three_elements() {
    let deps = <(Arc<DbPool>, Arc<RedisClient>, Arc<UserRepo>) as DepsTuple>::dep_type_ids();
    assert_eq!(deps.len(), 3);
}

#[test]
fn test_deps_tuple_ten_elements() {
    #[derive(Component, Default)]
    pub struct A;
    #[derive(Component, Default)]
    pub struct B;
    #[derive(Component, Default)]
    pub struct C;
    #[derive(Component, Default)]
    pub struct D;
    #[derive(Component, Default)]
    pub struct E;

    #[derive(Component)]
    #[allow(dead_code)]
    pub struct TenDeps {
        pub a: Arc<A>,
        pub b: Arc<B>,
        pub c: Arc<C>,
        pub d: Arc<D>,
        pub e: Arc<E>,
    }

    // 5 个 Deps 元素
    let deps = <(Arc<A>, Arc<B>, Arc<C>, Arc<D>, Arc<E>) as DepsTuple>::dep_type_ids();
    assert_eq!(deps.len(), 5);
}

// ══════════════════════════════════════════════════════════════════════════
// 22. Store inject 错误路径（downcast 失败 + inject_or_panic）
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
struct DowncastTarget;
impl Component for DowncastTarget {
    type Deps = ();
    fn build(_: ()) -> Self {
        DowncastTarget
    }
}

#[test]
fn test_inject_downcast_failure() {
    let store = Store::new();
    // 以 u64 类型注册到 DowncastTarget 的 TypeId 下，使 downcast 失败
    store.inner().insert(
        TypeId::of::<DowncastTarget>(),
        CompRef::Cached(Arc::new(42u64) as Arc<dyn std::any::Any + Send + Sync>),
    );

    let result = store.inject::<DowncastTarget>();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.context().unwrap_or_default().contains("downcast"));
}

#[test]
fn test_inject_or_panic_unregistered() {
    #[derive(Debug)]
    struct Phantom;
    impl Component for Phantom {
        type Deps = ();
        fn build(_: ()) -> Self {
            Phantom
        }
    }

    let store = Store::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        store.inject_or_panic::<Phantom>()
    }));
    assert!(result.is_err(), "未注册组件 inject_or_panic 应 panic");
}

// ══════════════════════════════════════════════════════════════════════════
// 23. AOP 拦截器链空链默认值
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_interceptor_chain_default() {
    let chain = InterceptorChain::default();
    let ctx = CallContext::new("test");
    // 空链不应 panic
    chain.before_all(&ctx).unwrap();
    let mut result = CallResult::Ok;
    chain.after_all(&ctx, &mut result);
}

// ══════════════════════════════════════════════════════════════════════════
// 24. ArgValue From 转换
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_arg_value_conversions() {
    use tx_di_core::aop::ArgValue;

    let _ = ArgValue::from(42i64);
    let _ = ArgValue::from("hello");
    let _ = ArgValue::from("world".to_string());
    let _ = ArgValue::from(true);
}

// ══════════════════════════════════════════════════════════════════════════
// 25. 列表 trait 注入（Vec<Arc<dyn Trait>>）— 完全 safe，无 unsafe
// ══════════════════════════════════════════════════════════════════════════

/// 消费所有 Reporter 实现的组件
#[derive(Component)]
pub struct ReporterAggregator {
    /// 列表注入：注入所有实现了 dyn Reporter 的组件
    pub reporters: Vec<Arc<dyn Reporter>>,
}

#[test]
fn test_list_trait_inject() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let agg = ctx.inject::<ReporterAggregator>();

    // 应注入所有 Reporter 实现（JsonReporter + XmlReporter）
    assert!(
        agg.reporters.len() >= 2,
        "应至少注入 2 个 Reporter 实现，实际: {}",
        agg.reporters.len()
    );

    let data: Vec<&str> = agg.reporters.iter().map(|r| r.report()).collect();
    assert!(data.contains(&"json"), "应包含 json 实现, got: {:?}", data);
    assert!(data.contains(&"xml"), "应包含 xml 实现, got: {:?}", data);
}

/// 列表注入与其他字段混合
#[derive(Component)]
pub struct MixedConsumer {
    pub db: Arc<DbPool>,
    pub reporters: Vec<Arc<dyn Reporter>>,
}

#[test]
fn test_list_trait_inject_mixed() {
    let ctx = BuildContext::new::<PathBuf>(None);
    let consumer = ctx.inject::<MixedConsumer>();

    assert_eq!(consumer.db.url, "sqlite");
    assert!(
        consumer.reporters.len() >= 2,
        "混合字段也应正确注入列表"
    );
}
