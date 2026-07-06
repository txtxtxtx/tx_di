//! SIP 消息中间件（拦截器）
//!
//! 提供洋葱模型的中间件链，作用在 [`SipTx`](crate::sip_tx::SipTx) 上，
//! 在 SIP 消息到达 handler 前后执行横切逻辑，例如：认证、日志、NAT 修正、速率限制等。
//!
//! ## 收集方式（不再使用全局 REGISTRY）
//!
//! 中间件通过 DI 收集：实现 [`SipMiddleware`] 并用
//! `#[tx_comp(as_trait = dyn SipMiddleware)]` 注册为组件，
//! 由 `SipPlugin::app_async_init` 在启动时用
//! `inject_all_traits_from_store::<dyn SipMiddleware>()` 收集并注入 [`SipRouter`](crate::handler::SipRouter)。
//! 这样每个 App 实例拥有独立的中间件集合，避免全局状态导致的串台与不可测。
//!
//! ## 执行顺序
//!
//! 按 `sort` 升序组成一个洋葱链：
//!
//! ```text
//! 外层中间件 A  →  外层中间件 B  →  ...  →  handler  →  ...  →  外层中间件 B  →  外层中间件 A
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::sip_tx::SipTx;
use tx_di_core::RIE;

/// 下一个中间件/处理器的调用函数
pub type SipNextFn = Box<dyn FnOnce(SipTx) -> SipNextFut + Send>;

/// `SipNextFn` 返回的 Future 类型
pub type SipNextFut = Pin<Box<dyn Future<Output = RIE<()>> + Send>>;

/// SIP 中间件 trait
///
/// 实现此 trait 并用 `#[tx_comp(as_trait = dyn SipMiddleware)]` 注册，
/// 即可被 `SipPlugin` 自动收集进中间件链。
///
/// # 执行模型
///
/// - **pre**：调用 `next(tx)` 之前，可 `tx.request()` 检视请求、可 `tx.reply()` 短路
/// - **next**：`next(tx).await` 把事务交给内层中间件（或最终 handler）；不调用则不继续
/// - **post**：`next(tx).await` 之后，可基于 `tx.replied()` 等做日志/指标（post-step 仍持有 `tx`）
#[async_trait::async_trait]
pub trait SipMiddleware: Send + Sync {
    /// 处理 SIP 事务
    ///
    /// - `tx`：共享事务信封（`SipTx`），可读 `tx.request()`、可 `tx.reply()` 短路
    /// - `next`：调用链中的下一个中间件（或最终 handler）；仅可调用一次
    async fn process(&self, tx: SipTx, next: SipNextFn) -> RIE<()>;

    /// 排序（数值越小越外层，越先执行），默认 100
    fn sort(&self) -> i32 {
        100
    }

    /// 中间件名称（日志/调试）
    fn name(&self) -> &str;
}

/// 收集并排序中间件，构建洋葱链
///
/// `mws` 内部按 `sort` 升序排列（外层在前）。`handler` 为链的最内层（最终处理器）。
///
/// 返回 `FnOnce(SipTx) -> SipNextFut`：`SipRouter::dispatch` 调用一次即可触发整条链。
pub(crate) fn build_chain(
    mws: &[Arc<dyn SipMiddleware>],
    handler: crate::handler::SipHandlerFn,
) -> impl FnOnce(SipTx) -> SipNextFut {
    // 按 sort 升序（外层在前）
    let mut sorted: Vec<Arc<dyn SipMiddleware>> = mws.to_vec();
    sorted.sort_by_key(|m| m.sort());

    let mut next: SipNextFn = Box::new(move |tx| (handler)(tx));

    // 逆序包裹 → 正序执行（洋葱模型）
    for mw in sorted.iter().rev() {
        let mw = mw.clone();
        let parent = next;
        next = Box::new(move |tx| {
            let mw = mw.clone();
            Box::pin(async move { mw.process(tx, parent).await })
        });
    }
    move |tx| (next)(tx)
}
