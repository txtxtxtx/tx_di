//! AOP 拦截器 — 横切关注点分离
//!
//! 拦截器链存储在组件实例自身的隐藏字段中（`OnceLock<Arc<InterceptorChain>>`），
//! 不依赖全局表，无锁、无泄漏、无 ABA 风险。
//!
//! # 使用方式
//!
//! ```ignore
//! #[derive(Component)]
//! pub struct AuthInterceptor { pub session: Arc<SessionService> }
//! impl Interceptor for AuthInterceptor {
//!     fn before(&self, ctx: &CallContext) -> RIE<()> { Ok(()) }
//! }
//!
//! #[derive(Component)]
//! #[component(intercept(AuthInterceptor))]
//! pub struct UserService;
//!
//! impl UserService {
//!     #[intercept]
//!     pub fn get_user(&self, user_id: u64) -> RIE<User> { ... }
//! }
//! ```

use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::RIE;
use crate::component::Component;
use crate::store::Store;

// ── ArgValue ────────────────────────────────────────────────────────────────

/// 参数值（用于日志和调试，支持 serde 反序列化恢复复杂类型）
#[derive(Debug, Clone)]
pub enum ArgValue {
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Bool(bool),
    /// 任意类型的 serde JSON 表示
    Serialized {
        type_id: TypeId,
        type_name: &'static str,
        json: String,
    },
}

impl From<i64> for ArgValue {
    fn from(v: i64) -> Self {
        ArgValue::I64(v)
    }
}
impl From<u64> for ArgValue {
    fn from(v: u64) -> Self {
        ArgValue::U64(v)
    }
}
impl From<f64> for ArgValue {
    fn from(v: f64) -> Self {
        ArgValue::F64(v)
    }
}
impl From<&str> for ArgValue {
    fn from(v: &str) -> Self {
        ArgValue::Str(v.to_string())
    }
}
impl From<String> for ArgValue {
    fn from(v: String) -> Self {
        ArgValue::Str(v)
    }
}
impl From<bool> for ArgValue {
    fn from(v: bool) -> Self {
        ArgValue::Bool(v)
    }
}

// ── CallContext ─────────────────────────────────────────────────────────────

/// 调用上下文 — 传递给拦截器的上下文信息
#[derive(Clone)]
pub struct CallContext {
    /// 方法名
    pub method_name: &'static str,
    /// 命名参数列表
    pub args: Vec<(/* name */ &'static str, ArgValue)>,
}

impl CallContext {
    pub fn new(method_name: &'static str) -> Self {
        CallContext {
            method_name,
            args: Vec::new(),
        }
    }

    /// 添加命名参数
    pub fn with_arg(mut self, name: &'static str, val: ArgValue) -> Self {
        self.args.push((name, val));
        self
    }

    /// 按名称查找参数
    pub fn get_arg(&self, name: &str) -> Option<&ArgValue> {
        self.args.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }
}

// ── CallResult ──────────────────────────────────────────────────────────────

/// 调用结果（`after` / `around` 可读取或修改）
#[derive(Debug)]
pub enum CallResult {
    Ok,
    /// 错误消息字符串
    Err(String),
}

impl CallResult {
    /// 转换为 `RIE<()>`，供调用方传播
    pub fn into_result(self) -> RIE<()> {
        match self {
            CallResult::Ok => Ok(()),
            CallResult::Err(msg) => Err(crate::error::AppError::with_context(
                crate::error::DiErr::InjectError,
                msg,
            )),
        }
    }
}

// ── CallFn ──────────────────────────────────────────────────────────────────

/// 可执行的业务调用（类似 `FnOnce()`, 用于 `around` 包装）
pub trait CallFn: Send {
    fn execute(self: Box<Self>) -> CallResult;
}

impl<F: FnOnce() -> CallResult + Send> CallFn for F {
    fn execute(self: Box<Self>) -> CallResult {
        self()
    }
}

pub type BoxCall = Box<dyn CallFn>;

// ── Interceptor trait ───────────────────────────────────────────────────────

/// AOP 拦截器 trait
///
/// - `before`：只读上下文，返回 `Err` 阻止方法执行
/// - `after`：可读取 `CallResult`
/// - `around`：完全包裹调用，可短路 / 重试 / 替换返回
pub trait Interceptor: Send + Sync + 'static {
    #[allow(unused_variables)]
    fn before(&self, ctx: &CallContext) -> RIE<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn after(&self, ctx: &CallContext, result: &CallResult) {}

    /// around 拦截（默认委托给 call 执行业务逻辑）
    fn around(&self, _ctx: &CallContext, call: BoxCall) -> CallResult {
        call.execute()
    }
}

// ── InterceptorChain ────────────────────────────────────────────────────────

/// 拦截器链 — 按顺序执行多个拦截器
///
/// 执行流程：`before_all → around_all（最内层为业务 body）→ after_all`
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    pub fn new() -> Self {
        InterceptorChain {
            interceptors: Vec::new(),
        }
    }

    /// 添加拦截器（按值，自动 `Arc<dyn Interceptor>`）
    pub fn push<I: Interceptor>(&mut self, interceptor: I) {
        self.interceptors.push(Arc::new(interceptor));
    }

    /// 添加已 `Arc` 包装的拦截器
    pub fn push_arc(&mut self, interceptor: Arc<dyn Interceptor>) {
        self.interceptors.push(interceptor);
    }

    /// `before_all` — 顺序执行，任一 Err 即停止
    pub fn before_all(&self, ctx: &CallContext) -> RIE<()> {
        for ic in &self.interceptors {
            ic.before(ctx)?;
        }
        Ok(())
    }

    /// `after_all` — 逆序执行
    pub fn after_all(&self, ctx: &CallContext, result: &CallResult) {
        for ic in self.interceptors.iter().rev() {
            ic.after(ctx, result);
        }
    }

    /// `around_all` — 构建洋葱调用链
    ///
    /// 最外层拦截器（声明顺序的第一个）最先执行 `around`，
    /// 逐层包裹直至最内层的 `call`（业务 body）。
    /// 闭包捕获 `method_name` 副本（`'static`）以满足 `Send + 'static` 约束。
    pub fn around_all(&self, ctx: &CallContext, exec: BoxCall) -> CallResult {
        let method_name = ctx.method_name;
        let args = ctx.args.clone();
        let mut call = exec;
        for ic in self.interceptors.iter().rev() {
            let inner = call;
            let ic = Arc::clone(ic);
            let mn = method_name;
            let a = args.clone();
            call = Box::new(move || {
                let ctx = CallContext {
                    method_name: mn,
                    args: a,
                };
                ic.around(&ctx, inner)
            });
        }
        call.execute()
    }
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}

// ── 内置拦截器 ──────────────────────────────────────────────────────────────

/// 日志拦截器
pub struct LoggingInterceptor;

impl Component for LoggingInterceptor {
    type Deps = ();
    fn build(_: Self::Deps, _store: &Store) -> Self {
        LoggingInterceptor
    }
    const SCOPE: crate::Scope = crate::Scope::Singleton;
}
impl Default for LoggingInterceptor {
    fn default() -> Self {
        LoggingInterceptor
    }
}
impl Interceptor for LoggingInterceptor {
    fn before(&self, ctx: &CallContext) -> RIE<()> {
        tracing::info!(
            "→ {} {:?}",
            ctx.method_name,
            ctx.args
                .iter()
                .map(|(n, v)| format!("{}={:?}", n, v))
                .collect::<Vec<_>>()
        );
        Ok(())
    }
    fn after(&self, ctx: &CallContext, result: &CallResult) {
        match result {
            CallResult::Ok => tracing::info!("← {} OK", ctx.method_name),
            CallResult::Err(e) => tracing::warn!("← {} ERR: {}", ctx.method_name, e),
        }
    }
}

/// 指标拦截器
pub struct MetricsInterceptor {
    pub counter: AtomicU64,
}

impl Component for MetricsInterceptor {
    type Deps = ();
    fn build(_: Self::Deps, _store: &Store) -> Self {
        MetricsInterceptor {
            counter: AtomicU64::new(0),
        }
    }
    const SCOPE: crate::Scope = crate::Scope::Singleton;
}
impl MetricsInterceptor {
    pub fn new() -> Self {
        MetricsInterceptor {
            counter: AtomicU64::new(0),
        }
    }
    pub fn count(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }
}
impl Default for MetricsInterceptor {
    fn default() -> Self {
        Self::new()
    }
}
impl Interceptor for MetricsInterceptor {
    fn before(&self, _ctx: &CallContext) -> RIE<()> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
