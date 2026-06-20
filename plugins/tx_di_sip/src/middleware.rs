//! SIP 消息中间件（拦截器）
//!
//! 提供洋葱模型的中间件链，在 SIP 消息到达 handler 前后执行横切逻辑，
//! 例如：认证、日志、NAT 修正、速率限制等。
//!
//! 设计参考 `tx_di_axum` 的 `DynMiddleware` + `LAYER_REGISTRY` 模式。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use rsipstack::transaction::transaction::Transaction;

use crate::handler::SipHandlerFn;

/// SIP 中间件 trait
///
/// 实现此 trait 并调用 [`add_sip_middleware`] 注册，即可在消息分发前/后插入横切逻辑。
///
/// # 执行顺序
///
/// 注册时指定 `sort` 参数（数值越小越先执行），所有中间件按升序组成一个洋葱链：
///
/// ```text
/// 外层中间件 A  →  外层中间件 B  →  ...  →  handler  →  ...  →  外层中间件 B  →  外层中间件 A
/// ```
///
/// # 示例
///
/// ```rust,no_run
/// use tx_di_sip::{SipMiddleware, SipNextFn};
/// use rsipstack::transaction::transaction::Transaction;
/// use tx_di_core::RIE;
///
/// struct LoggingMiddleware;
///
/// #[async_trait::async_trait]
/// impl SipMiddleware for LoggingMiddleware {
///     async fn process(&self, tx: Transaction, next: SipNextFn) -> RIE<()> {
///         println!("收到 SIP 消息: {}", tx.original.method);
///         let result = next(tx).await;
///         println!("SIP 消息处理完成");
///         result
///     }
///
///     fn name(&self) -> &str { "LoggingMiddleware" }
/// }
///
/// tx_di_sip::add_sip_middleware(LoggingMiddleware, 0);
/// ```
#[async_trait::async_trait]
pub trait SipMiddleware: Send + Sync {
    /// 处理 SIP 事务
    ///
    /// - `tx`: 入站 SIP 事务
    /// - `next`: 调用链中的下一个中间件（或最终 handler）
    async fn process(&self, tx: Transaction, next: SipNextFn) -> tx_di_core::RIE<()>;

    /// 中间件名称（用于日志/调试）
    fn name(&self) -> &str;
}

/// 下一个中间件的调用函数
///
/// 接收 `Transaction`，返回 `Future<Output = RIE<()>>`。
pub type SipNextFn = Box<dyn FnOnce(Transaction) -> SipNextFut + Send>;

/// `SipNextFn` 返回的 Future 类型
pub type SipNextFut = Pin<Box<dyn Future<Output = tx_di_core::RIE<()>> + Send>>;

// ── 全局注册表 ────────────────────────────────────────────────────────────────

use std::sync::RwLock;

/// 排序条目：`(sort, name, middleware)`
type SortEntry = (i32, String, Arc<dyn SipMiddleware>);

/// 全局 SIP 中间件注册表
///
/// 使用 `RwLock<Vec<SortEntry>>` 支持运行时动态注册，
/// 读取时按 `sort` 升序遍历构建洋葱链。
static REGISTRY: once_cell::sync::Lazy<Arc<RwLock<Vec<SortEntry>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

/// 注册一个 SIP 中间件
///
/// - `middleware`: 中间件实例
/// - `sort`: 优先级（数值越小越外层，越先执行）
///
/// # 示例
///
/// ```rust,no_run
/// tx_di_sip::add_sip_middleware(MyMiddleware, 0);
/// ```
pub fn add_sip_middleware<M: SipMiddleware + 'static>(middleware: M, sort: i32) {
    let entry = (
        sort,
        middleware.name().to_string(),
        Arc::new(middleware) as Arc<dyn SipMiddleware>,
    );
    let mut registry = REGISTRY.write().expect("REGISTRY write lock");
    registry.push(entry);
    // 按 sort 升序排列，保证执行顺序稳定
    registry.sort_by_key(|(s, _, _)| *s);
}

/// 构建中间件链并执行
///
/// 内部使用，由 `SipRouter::dispatch()` 调用。
///
/// # 链构建逻辑
///
/// 注册表按 sort 升序排列后，依次包裹 handler：
///
/// ```text
/// middleware[0] → middleware[1] → ... → middleware[n-1] → handler
/// ```
///
/// 执行时从最外层（sort 最小）开始，形成洋葱模型。
pub(crate) fn apply_middleware_chain(
    tx: Transaction,
    handler: SipHandlerFn,
) -> Pin<Box<dyn Future<Output = tx_di_core::RIE<()>> + Send>> {
    let registry = REGISTRY.read().expect("REGISTRY read lock").clone();

    // 从最内层（handler）开始，逐层向外包裹
    let mut next: SipNextFn = Box::new(|tx| Box::pin((handler)(tx)));

    // 逆序遍历（从最内层中间件到最外层），构建洋葱链
    for (_sort, _name, mw) in registry.iter().rev() {
        let mw = mw.clone();
        let parent_next = next;
        next = Box::new(move |tx| {
            let mw = mw.clone();
            Box::pin(async move { mw.process(tx, parent_next).await })
        });
    }

    // 执行最外层中间件，触发整个链
    (next)(tx)
}
