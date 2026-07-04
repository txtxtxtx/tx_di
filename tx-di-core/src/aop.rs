//! AOP 拦截器 — 横切关注点分离
//!
//! 设计参考 `tower::Layer`：
//! - `Interceptor` trait 定义拦截逻辑
//! - `#[intercept]` 宏生成代理结构体
//! - 零运行时开销（静态分发）
//!
//! # 使用方式
//!
//! ```ignore
//! #[derive(Component)]
//! #[component(intercept(LoggingInterceptor))]
//! pub struct UserService {
//!     repo: Arc<UserRepo>,
//! }
//!
//! impl UserService {
//!     #[intercept]
//!     pub async fn get_user(&self, id: u64) -> Result<User, Error> {
//!         self.repo.find_by_id(id).await
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

/// 调用上下文 — 传递给拦截器的上下文信息
#[derive(Debug, Clone)]
pub struct CallContext {
    /// 方法名
    pub method_name: &'static str,
    /// 参数描述（用于日志）
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

    /// 添加参数
    pub fn with_arg(mut self, arg: ArgValue) -> Self {
        self.args.push(arg);
        self
    }
}

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

/// 调用结果
#[derive(Debug)]
pub enum CallResult {
    /// 成功
    Ok,
    /// 失败
    Err(String),
}

/// AOP 拦截器 trait
///
/// 实现此 trait 的类型可以作为拦截器，在方法调用前后执行逻辑。
///
/// # 示例
///
/// ```ignore
/// pub struct LoggingInterceptor;
///
/// impl Interceptor for LoggingInterceptor {
///     fn before(&self, ctx: &CallContext) {
///         tracing::info!("→ {} ({:?})", ctx.method_name, ctx.args);
///     }
///
///     fn after(&self, ctx: &CallContext, result: &CallResult) {
///         tracing::info!("← {} {:?}", ctx.method_name, result);
///     }
/// }
/// ```
pub trait Interceptor: Send + Sync + 'static {
    /// 调用前执行
    #[allow(unused_variables)]
    fn before(&self, ctx: &CallContext) {}

    /// 调用后执行
    #[allow(unused_variables)]
    fn after(&self, ctx: &CallContext, result: &CallResult) {}
}

/// Next 不再持有 dyn Interceptor 引用
pub struct Next<'a, C> {
    pub inner: &'a C,
}

impl<'a, C> Next<'a, C> {
    /// 创建新的 Next
    pub fn new(inner: &'a C) -> Self {
        Next { inner }
    }
}

/// 拦截器链 — 按顺序执行多个拦截器
pub struct InterceptorChain<I: Interceptor> {
    interceptors: Vec<I>,
}

impl<I: Interceptor> InterceptorChain<I> {
    /// 创建空链
    pub fn new() -> Self {
        InterceptorChain {
            interceptors: Vec::new(),
        }
    }

    /// 添加拦截器
    pub fn push(&mut self, interceptor: I) {
        self.interceptors.push(interceptor);
    }

    /// 调用前 — 按顺序执行所有拦截器的 before
    pub fn before_all(&self, ctx: &CallContext) {
        for interceptor in &self.interceptors {
            interceptor.before(ctx);
        }
    }

    /// 调用后 — 逆序执行所有拦截器的 after
    pub fn after_all(&self, ctx: &CallContext, result: &CallResult) {
        for interceptor in self.interceptors.iter().rev() {
            interceptor.after(ctx, result);
        }
    }
}

impl<I: Interceptor> Default for InterceptorChain<I> {
    fn default() -> Self {
        Self::new()
    }
}

// ── 常用拦截器实现 ────────────────────────────────────────────────────────

/// 日志拦截器 — 记录方法调用前后
pub struct LoggingInterceptor;

impl Interceptor for LoggingInterceptor {
    fn before(&self, ctx: &CallContext) {
        tracing::info!("→ {} {:?}", ctx.method_name, ctx.args);
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

impl MetricsInterceptor {
    /// 创建新的指标拦截器
    pub fn new() -> Self {
        MetricsInterceptor {
            counter: AtomicU64::new(0),
        }
    }

    /// 获取调用次数
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
    fn before(&self, _ctx: &CallContext) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}
