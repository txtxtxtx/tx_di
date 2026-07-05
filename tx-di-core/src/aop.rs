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
//!     fn before(&self, ctx: &mut CallContext) -> RIE<()> {
//!         let token: &str = ctx.get_raw_mut::<String>(0)
//!             .ok_or_else(|| anyhow!("missing token"))?;
//!         let user = self.session.validate(token)?;
//!         *ctx.get_raw_mut::<u64>(0).unwrap() = user.id;
//!         Ok(())
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

use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::component::Component;
use crate::RIE;

/// 调用上下文 — 传递给拦截器的上下文信息
pub struct CallContext {
    /// 方法名
    pub method_name: &'static str,
    /// 参数 Debug 表示（用于日志/监控拦截器）
    pub args: Vec<ArgValue>,
    /// 原始参数引用（用于业务拦截器通过 Any downcast 访问/修改）
    pub raw_args: Vec<Box<dyn Any + Send + Sync>>,
}

impl CallContext {
    /// 创建新的调用上下文
    pub fn new(method_name: &'static str) -> Self {
        CallContext {
            method_name,
            args: Vec::new(),
            raw_args: Vec::new(),
        }
    }

    /// 添加参数（Debug 表示）
    pub fn with_arg(mut self, arg: ArgValue) -> Self {
        self.args.push(arg);
        self
    }

    /// 添加原始参数值（拦截器可通过 `get_raw_mut` 修改）
    pub fn with_raw<T: Any + Send + Sync>(mut self, val: T) -> Self {
        self.raw_args.push(Box::new(val));
        self
    }

    /// 获取原始参数的可变引用（拦截器通过此方法覆写参数值）
    pub fn get_raw_mut<T: Any>(&mut self, index: usize) -> Option<&mut T> {
        self.raw_args.get_mut(index)?.downcast_mut::<T>()
    }
}

// ── ArgValue：日志用参数表示 ────────────────────────────────────────────────

/// 参数值（用于日志和调试）
#[derive(Debug, Clone)]
pub enum ArgValue {
    /// 整数
    I64(i64),
    /// 字符串
    Str(String),
    /// 布尔
    Bool(bool),
    /// 其他（Display）
    Other(String),
}

impl From<i64> for ArgValue {
    fn from(v: i64) -> Self {
        ArgValue::I64(v)
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

// ── CallResult：拦截器的 after 回调接收的结果 ───────────────────────────────

/// 调用结果
#[derive(Debug)]
pub enum CallResult {
    /// 成功
    Ok,
    /// 失败
    Err(String),
}

// ── Interceptor trait ──────────────────────────────────────────────────────

/// AOP 拦截器 trait
///
/// 实现此 trait 的类型可以拦截方法调用，在方法前后执行逻辑。
/// 拦截器应同时标记 `#[derive(Component)]` 以通过 DI 获取依赖。
///
/// # before 返回值
///
/// - `Ok(())` → 继续执行方法
/// - `Err(e)` → 阻止方法执行（由 `#[intercept]` 生成的代码转换为 panic）
///
/// # 参数覆写
///
/// 拦截器可通过 `ctx.get_raw_mut::<T>(index)` 修改方法参数值，
/// `#[intercept]` 生成的代码会在 `before_all` 后提取被覆写的参数传入业务方法。
pub trait Interceptor: Any + Send + Sync {
    /// 调用前执行
    /// - 可修改 `ctx` 中的参数（通过 `get_raw_mut`）
    /// - 返回 `Err` 将阻止方法执行
    #[allow(unused_variables)]
    fn before(&self, ctx: &mut CallContext) -> RIE<()> {
        Ok(())
    }

    /// 调用后执行
    #[allow(unused_variables)]
    fn after(&self, ctx: &CallContext, result: &CallResult) {}
}

// ── InterceptorChain ───────────────────────────────────────────────────────

/// 拦截器链 — 按顺序执行多个拦截器（非泛型，支持异构拦截器混合）
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    /// 创建空链
    pub fn new() -> Self {
        InterceptorChain {
            interceptors: Vec::new(),
        }
    }

    /// 添加拦截器（按值，自动包装为 `Arc<dyn Interceptor>`）
    pub fn push<I: Interceptor>(&mut self, interceptor: I) {
        self.interceptors.push(Arc::new(interceptor));
    }

    /// 添加已包装的拦截器
    pub fn push_arc(&mut self, interceptor: Arc<dyn Interceptor>) {
        self.interceptors.push(interceptor);
    }

    /// 调用前 — 按顺序执行所有拦截器的 before，任一返回 Err 则立即停止
    pub fn before_all(&self, ctx: &mut CallContext) -> RIE<()> {
        for interceptor in &self.interceptors {
            interceptor.before(ctx)?;
        }
        Ok(())
    }

    /// 调用后 — 逆序执行所有拦截器的 after
    pub fn after_all(&self, ctx: &CallContext, result: &CallResult) {
        for interceptor in self.interceptors.iter().rev() {
            interceptor.after(ctx, result);
        }
    }
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}

// ── 内置拦截器 ─────────────────────────────────────────────────────────────

/// 日志拦截器 — 记录方法调用前后
pub struct LoggingInterceptor;

impl Component for LoggingInterceptor {
    type Deps = ();
    fn build(_: Self::Deps) -> Self { LoggingInterceptor }
    const SCOPE: crate::Scope = crate::Scope::Singleton;
}

impl Default for LoggingInterceptor {
    fn default() -> Self { LoggingInterceptor }
}

impl Interceptor for LoggingInterceptor {
    fn before(&self, ctx: &mut CallContext) -> RIE<()> {
        tracing::info!("→ {} {:?}", ctx.method_name, ctx.args);
        Ok(())
    }

    fn after(&self, ctx: &CallContext, result: &CallResult) {
        match result {
            CallResult::Ok => tracing::info!("← {} OK", ctx.method_name),
            CallResult::Err(e) => tracing::warn!("← {} ERR: {}", ctx.method_name, e),
        }
    }
}

/// 指标拦截器 — 统计方法调用次数
pub struct MetricsInterceptor {
    pub counter: AtomicU64,
}

impl Component for MetricsInterceptor {
    type Deps = ();
    fn build(_: Self::Deps) -> Self {
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
    fn before(&self, _ctx: &mut CallContext) -> RIE<()> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
