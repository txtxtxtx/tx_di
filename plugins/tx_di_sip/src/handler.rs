//! SIP 消息处理器注册中心
//!
//! 提供类似 axum 路由表的消息处理机制：
//! 用户可在应用启动前通过 [`SipRouter::add_handler`] 注册各种方法的处理函数，
//! SIP 服务启动后会将收到的消息分发给对应的处理器。
//!
//! ## 性能优化
//!
//! - 查找使用 DashMap 索引：精确匹配 O(1)，无需每消息排序
//! - 索引在注册时构建，运行时只读查询，完全无锁

use dashmap::DashMap;
use rsipstack::transaction::transaction::Transaction;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, RwLock};
use tracing::{error, info};

/// 异步 SIP 消息处理函数类型
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

/// 全局处理器注册表（用于支持运行时遍历和移除）
static HANDLER_REGISTRY: LazyLock<Arc<RwLock<Vec<HandlerEntry>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// 方法名 → HandlerEntry 索引（按 priority 升序排列）
/// 查找复杂度 O(1)，每条消息分发时直接命中
static HANDLER_INDEX: LazyLock<Arc<DashMap<String, Vec<HandlerEntry>>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

/// SIP 路由器
///
/// 管理 SIP 消息处理器的注册与分发，设计上与 `WebPlugin::add_router` 对称。
///
/// # 性能特性
///
/// - 注册（`add_handler`）：O(1) 写入，同时更新注册表和索引
/// - 查找（`dispatch`）：O(1) 精确匹配（DashMap），无锁并发读取
/// - Fallback：O(n) 遍历 catch-all（仅在无精确匹配时触发）
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
        let handler_fn: SipHandlerFn = Arc::new(move |tx| Box::pin(handler(tx)));

        let entry = HandlerEntry {
            method: method_str.clone(),
            priority,
            handler: handler_fn.clone(),
        };

        // 1. 写入注册表
        let mut registry = HANDLER_REGISTRY.write().expect("handler registry write lock");
        registry.push(HandlerEntry {
            method: method_str.clone(),
            priority,
            handler: handler_fn,
        });
        drop(registry); // 尽快释放写锁

        // 2. 同步更新索引（按 priority 插入排序）
        if let Some(ref method_name) = method_str {
            let mut entries = HANDLER_INDEX
                .entry(method_name.clone())
                .or_insert_with(Vec::new);

            // 二分查找插入位置，维持 priority 升序
            let pos = entries
                .binary_search_by_key(&priority, |e| e.priority)
                .unwrap_or_else(|pos| pos);
            entries.insert(pos, entry);
        }

        info!(
            method = ?method_str,
            priority = priority,
            "SIP 处理器已注册（O(1) 索引）"
        );
    }

    /// 分发消息：O(1) 查找匹配处理器并调用
    ///
    /// 查找策略：
    /// 1. DashMap 精确匹配（O(1)，无锁）→ 取最低 priority 条目
    /// 2. 若无精确匹配，遍历注册表查找 catch-all（O(n)，持有读锁）
    pub(crate) async fn dispatch(tx: Transaction) {
        let method = tx.original.method.to_string().to_uppercase();

        let handler = {
            // 路径 A：O(1) 精确查找（DashMap 并发读，无锁）
            if let Some(entries) = HANDLER_INDEX.get(&method) && let Some(entry) = entries.first() {
                return Self::invoke_handler(entry.handler.clone(), tx).await;
            }

            // 路径 B：遍历 catch-all（O(n)，持有读锁）
            // 仅当无精确匹配时触发，正常情况下极少见
            let registry = HANDLER_REGISTRY.read().unwrap_or_else(|e| {
                error!(error = %e, "HANDLER_REGISTRY 读锁中毒，尝试恢复");
                e.into_inner()
            });
            registry
                .iter()
                .find(|e| e.method.is_none())
                .map(|e| e.handler.clone())
        };

        match handler {
            Some(h) => {
                Self::invoke_handler(h, tx).await;
            }
            None => {
                // 没有注册任何处理器时自动回复 405
                if let Err(e) = default_handler(tx).await {
                    error!(error = %e, "SIP 默认处理器执行出错");
                }
            }
        }
    }

    /// 调用 handler 并统一处理错误日志
    async fn invoke_handler(handler: SipHandlerFn, tx: Transaction) {
        if let Err(e) = handler(tx).await {
            error!(error = %e, "SIP 处理器执行出错");
        }
    }

    /// 清空所有已注册的处理器（主要用于测试）
    pub fn clear() {
        let mut registry = HANDLER_REGISTRY.write().expect("handler registry write lock");
        registry.clear();
        // 清空 DashMap 索引
        HANDLER_INDEX.clear();
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

/// 默认处理器：对没有注册处理器的方法回复 405 Method Not Allowed 方法不允许
async fn default_handler(mut tx: Transaction) -> anyhow::Result<()> {
    use rsipstack::sip::StatusCode;
    info!(method = %tx.original.method, "收到未注册方法，回复 405");
    tx.reply(StatusCode::MethodNotAllowed)
        .await
        .map_err(|e| anyhow::anyhow!("回复 405 失败: {}", e))?;
    Ok(())
}
