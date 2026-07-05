//! AOP 拦截器 — 横切关注点分离
//!
//! 拦截器通过 DI 框架管理：拦截器本身也是 `#[derive(Component)]`，
//! 可依赖其他服务。通过 `#[component(intercept(InterceptorType))]` 显式声明
//! 需要哪些拦截器，框架在 App 阶段从 Store 中精确注入。
//!
//! # 使用方式
//!
//! ```ignore
//! // 1. 定义拦截器（也是 DI 组件）
//! #[derive(Component)]
//! pub struct AuthInterceptor {
//!     pub session: Arc<SessionService>,   // DI 自动注入
//! }
//! impl Interceptor for AuthInterceptor {
//!     fn before(&self, ctx: &CallContext) -> RIE<()> {
//!         tracing::info!("参数: {:?}", ctx.args);
//!         Ok(())
//!     }
//!     fn after(&self, ctx: &CallContext, result: &mut CallResult) {
//!         match result {
//!             CallResult::Ok => tracing::info!("成功"),
//!             CallResult::Err(e) => tracing::warn!("失败: {}", e),
//!         }
//!     }
//! }
//!
//! // 2. 业务组件声明需要哪些拦截器
//! #[derive(Component)]
//! #[component(intercept(AuthInterceptor, AuditInterceptor))]
//! pub struct UserService;
//!
//! impl UserService {
//!     #[intercept]
//!     pub fn get_user(&self, user_id: u64) -> RIE<User> {
//!         // 业务逻辑
//!     }
//! }
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::component::Component;
use crate::RIE;

// ── CallContext ─────────────────────────────────────────────────────────────

/// 调用上下文 — 传递给拦截器的上下文信息
pub struct CallContext {
    /// 方法名
    pub method_name: &'static str,
    /// 参数 Debug 表示（用于日志/监控拦截器）
    pub args: Vec<ArgValue>,
}

impl CallContext {
    /// 创建新的调用上下文
    pub fn new(method_name: &'static str) -> Self {
        CallContext {
            method_name,
            args: Vec::new(),
        }
    }
    /// 添加参数（Debug 表示）
    pub fn with_arg(mut self, arg: ArgValue) -> Self {
        self.args.push(arg);
        self
    }
}

// ── ArgValue ────────────────────────────────────────────────────────────────

/// 参数值（用于日志和调试）
#[derive(Debug, Clone)]
pub enum ArgValue {
    I64(i64),
    Str(String),
    Bool(bool),
    Other(String),
}

impl From<i64> for ArgValue {
    fn from(v: i64) -> Self { ArgValue::I64(v) }
}
impl From<&str> for ArgValue {
    fn from(v: &str) -> Self { ArgValue::Str(v.to_string()) }
}
impl From<String> for ArgValue {
    fn from(v: String) -> Self { ArgValue::Str(v) }
}
impl From<bool> for ArgValue {
    fn from(v: bool) -> Self { ArgValue::Bool(v) }
}

// ── CallResult ──────────────────────────────────────────────────────────────

/// 调用结果（`after` 可修改此值以加工返回描述）
#[derive(Debug)]
pub enum CallResult {
    Ok,
    Err(String),
}

// ── Interceptor trait ───────────────────────────────────────────────────────

/// AOP 拦截器 trait
///
/// - `before`：只读上下文，返回 `Err` 阻止方法执行
/// - `after`：可修改 `CallResult` 以加工返回描述（日志/监控用）
pub trait Interceptor: Send + Sync + 'static {
    #[allow(unused_variables)]
    fn before(&self, ctx: &CallContext) -> RIE<()> { Ok(()) }
    #[allow(unused_variables)]
    fn after(&self, ctx: &CallContext, result: &mut CallResult) {}
}

// ── InterceptorChain ────────────────────────────────────────────────────────

/// 拦截器链 — 按顺序执行多个拦截器（非泛型，支持异构拦截器混合）
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    pub fn new() -> Self {
        InterceptorChain { interceptors: Vec::new() }
    }
    /// 添加拦截器（按值，自动 `Arc<dyn Interceptor>`）
    pub fn push<I: Interceptor>(&mut self, interceptor: I) {
        self.interceptors.push(Arc::new(interceptor));
    }
    /// 添加已 `Arc` 包装的拦截器
    pub fn push_arc(&mut self, interceptor: Arc<dyn Interceptor>) {
        self.interceptors.push(interceptor);
    }
    /// before_all — 顺序执行，任一 Err 即停止
    pub fn before_all(&self, ctx: &CallContext) -> RIE<()> {
        for interceptor in &self.interceptors {
            interceptor.before(ctx)?;
        }
        Ok(())
    }
    /// after_all — 逆序执行，可传递可变 `CallResult` 让拦截器加工
    pub fn after_all(&self, ctx: &CallContext, result: &mut CallResult) {
        for interceptor in self.interceptors.iter().rev() {
            interceptor.after(ctx, result);
        }
    }
}
impl Default for InterceptorChain { fn default() -> Self { Self::new() } }

// ── 内置拦截器 ──────────────────────────────────────────────────────────────

/// 日志拦截器
pub struct LoggingInterceptor;

impl Component for LoggingInterceptor {
    type Deps = ();
    fn build(_: Self::Deps) -> Self { LoggingInterceptor }
    const SCOPE: crate::Scope = crate::Scope::Singleton;
}
impl Default for LoggingInterceptor { fn default() -> Self { LoggingInterceptor } }
impl Interceptor for LoggingInterceptor {
    fn before(&self, ctx: &CallContext) -> RIE<()> {
        tracing::info!("→ {} {:?}", ctx.method_name, ctx.args);
        Ok(())
    }
    fn after(&self, ctx: &CallContext, result: &mut CallResult) {
        match result {
            CallResult::Ok => tracing::info!("← {} OK", ctx.method_name),
            CallResult::Err(e) => tracing::warn!("← {} ERR: {}", ctx.method_name, e),
        }
    }
}

/// 指标拦截器
pub struct MetricsInterceptor { pub counter: AtomicU64 }

impl Component for MetricsInterceptor {
    type Deps = ();
    fn build(_: Self::Deps) -> Self { MetricsInterceptor { counter: AtomicU64::new(0) } }
    const SCOPE: crate::Scope = crate::Scope::Singleton;
}
impl MetricsInterceptor {
    pub fn new() -> Self { MetricsInterceptor { counter: AtomicU64::new(0) } }
    pub fn count(&self) -> u64 { self.counter.load(Ordering::Relaxed) }
}
impl Default for MetricsInterceptor { fn default() -> Self { Self::new() } }
impl Interceptor for MetricsInterceptor {
    fn before(&self, _ctx: &CallContext) -> RIE<()> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
