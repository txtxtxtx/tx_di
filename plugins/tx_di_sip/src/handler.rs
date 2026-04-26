//! SIP 消息处理器注册中心
//!
//! 提供类似 axum 路由表的消息处理机制：
//! 用户可在应用启动前通过 [`SipRouter::add_handler`] 注册各种方法的处理函数，
//! SIP 服务启动后会将收到的消息分发给对应的处理器。

use rsipstack::transaction::transaction::Transaction;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, RwLock};
use tracing::{error, info};

/// 异步 SIP 消息处理函数类型
///
/// 接收一个 `Transaction`，返回 `anyhow::Result<()>`。
pub type SipHandlerFn = Arc<
    dyn Fn(Transaction) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>
        + Send
        + Sync,
>;

/// 消息处理器条目
pub struct HandlerEntry {
    /// 匹配的 SIP 方法名（大写），例如 `"REGISTER"`, `"INVITE"`, `"OPTIONS"`
    /// 若为 `None`，则匹配所有方法（catch-all）
    pub method: Option<String>,
    /// 优先级，值越小越先匹配（默认 0）
    pub priority: i32,
    /// 处理函数
    pub handler: SipHandlerFn,
}

/// 全局处理器注册表
static HANDLER_REGISTRY: LazyLock<Arc<RwLock<Vec<HandlerEntry>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// SIP 路由器
///
/// 管理 SIP 消息处理器的注册与分发，设计上与 `WebPlugin::add_router` 对称。
///
/// # 示例
///
/// ```rust,no_run
/// use tx_di_sip::SipRouter;
///
/// // 注册 REGISTER 消息处理器
/// SipRouter::add_handler(
///     Some("REGISTER"),
///     0,
///     |mut tx| async move {
///         println!("收到 REGISTER: {}", tx.original);
///         tx.reply(rsipstack::sip::StatusCode::OK).await?;
///         Ok(())
///     }
/// );
///
/// // 注册 catch-all 处理器（匹配所有方法）
/// SipRouter::add_handler(
///     None,
///     100,
///     |mut tx| async move {
///         println!("未知方法: {}", tx.original.method);
///         tx.reply(rsipstack::sip::StatusCode::MethodNotAllowed).await?;
///         Ok(())
///     }
/// );
/// ```
pub struct SipRouter;

impl SipRouter {
    /// 注册一个 SIP 消息处理器
    ///
    /// # 参数
    ///
    /// - `method` — 要匹配的 SIP 方法名（大写），传 `None` 则匹配所有方法
    /// - `priority` — 优先级，值越小越先检查（相同方法时有效）
    /// - `handler` — 异步处理函数，签名：`async fn(Transaction) -> anyhow::Result<()>`
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use tx_di_sip::SipRouter;
    ///
    /// SipRouter::add_handler(
    ///     Some("OPTIONS"),
    ///     0,
    ///     |mut tx| async move {
    ///         tx.reply(rsipstack::sip::StatusCode::OK).await?;
    ///         Ok(())
    ///     }
    /// );
    /// ```
    pub fn add_handler<M, F, Fut>(method: M, priority: i32, handler: F)
    where
        M: Into<Option<&'static str>>,
        F: Fn(Transaction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let method_str = method.into().map(|m| m.to_uppercase());
        let handler_fn: SipHandlerFn =
            Arc::new(move |tx| Box::pin(handler(tx)));
        let mut registry = HANDLER_REGISTRY.write().expect("handler registry write lock");
        registry.push(HandlerEntry {
            method: method_str.clone(),
            priority,
            handler: handler_fn,
        });
        info!(
            method = ?method_str,
            priority = priority,
            "SIP 处理器已注册"
        );
    }

    /// 分发消息：按优先级找到第一个匹配的处理器并调用
    ///
    /// 查找策略：
    /// 1. 优先按 `priority` 升序扫描
    /// 2. 精确匹配方法名
    /// 3. 若无精确匹配，使用 catch-all（`method == None`）
    pub(crate) async fn dispatch(tx: Transaction) {
        let method = tx.original.method.to_string().to_uppercase();

        // 查找精确匹配 + catch-all
        let handler = {
            let registry = HANDLER_REGISTRY.read().expect("handler registry read lock");
            // 先按 priority 排序，取第一个精确匹配
            let mut entries: Vec<&HandlerEntry> = registry.iter().collect();
            entries.sort_by_key(|e| e.priority);

            let exact = entries
                .iter()
                .find(|e| e.method.as_deref() == Some(method.as_str()))
                .map(|e| e.handler.clone());

            let fallback = entries
                .iter()
                .find(|e| e.method.is_none())
                .map(|e| e.handler.clone());

            exact.or(fallback)
        };

        match handler {
            Some(h) => {
                if let Err(e) = h(tx).await {
                    error!(error = %e, "SIP 处理器执行出错");
                }
            }
            None => {
                // 没有注册任何处理器时自动回复 405
                if let Err(e) = default_handler(tx).await {
                    error!(error = %e, "SIP 默认处理器执行出错");
                }
            }
        }
    }

    /// 清空所有已注册的处理器（主要用于测试）
    pub fn clear() {
        let mut registry = HANDLER_REGISTRY.write().expect("handler registry write lock");
        registry.clear();
        info!("已清空所有 SIP 处理器");
    }

    /// 获取已注册的处理器数量
    pub fn handler_count() -> usize {
        HANDLER_REGISTRY
            .read()
            .expect("handler registry read lock")
            .len()
    }
}

/// 默认处理器：对没有注册处理器的方法回复 405 Method Not Allowed
async fn default_handler(mut tx: Transaction) -> anyhow::Result<()> {
    use rsipstack::sip::StatusCode;
    info!(method = %tx.original.method, "收到未注册方法，回复 405");
    tx.reply(StatusCode::MethodNotAllowed)
        .await
        .map_err(|e| anyhow::anyhow!("回复 405 失败: {}", e))?;
    Ok(())
}
