use dashmap::DashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tracing::{error, info, warn};

// ---------- 从依赖中导入的类型 ----------
use rsipstack::sip::StatusCode;
use rsipstack::transaction::transaction::Transaction;
use tx_di_core::{IE, RIE};

// ---------- 异步处理器类型 ----------
/// SIP 消息异步处理函数（线程安全、可共享）
pub type SipHandlerFn = Arc<
    dyn Fn(Transaction) -> Pin<Box<dyn Future<Output = RIE<()>> + Send + 'static>>
    + Send
    + Sync,
>;

// ---------- 内部条目结构 ----------
/// 索引中存储的条目：优先级 + 处理器
type HandlerEntry = (i32, SipHandlerFn);

// ---------- SipRouter 定义 ----------
/// SIP 消息路由器
///
/// 支持按 SIP 方法名注册异步处理器，并支持 catch‑all 兜底。
/// 内部使用 `DashMap` 实现高并发、无锁读的 O(1) 分发。
///
/// # 特性
///
/// - 精确方法匹配（如 `"REGISTER"`, `"INVITE"`）和 catch‑all（`method: None`）
/// - 优先级排序：同方法内按 `priority` 升序执行
/// - 可运行时动态增删处理器
/// - 可定制默认处理器（未注册任何方法时的行为，默认为返回 405）
/// - 线程安全，可随意 Clone 并在多任务间共享
///
/// # 示例
///
/// ```rust,no_run
/// use sip_router::SipRouter;
///
/// let router = SipRouter::new();
///
/// // 注册 REGISTER 处理器
/// router.add_handler(
///     Some("REGISTER"),
///     0,
///     |mut tx| async move {
///         tx.reply(StatusCode::OK).await?;
///         Ok(())
///     },
/// );
///
/// // 注册 catch‑all
/// router.add_handler(
///     None,
///     100,
///     |mut tx| async move {
///         tx.reply(StatusCode::MethodNotAllowed).await?;
///         Ok(())
///     },
/// );
///
/// // 在服务中使用
/// // router.dispatch(transaction).await;
/// ```
#[derive(Clone)]
pub struct SipRouter {
    /// 方法名（大写）→ 按优先级排序的处理器列表
    /// 空字符串 `""` 作为 catch‑all 的键
    handlers: Arc<DashMap<String, Vec<HandlerEntry>>>,

    /// 可选的默认处理器（当没有任何处理器匹配时调用）
    default_handler: Arc<RwLock<Option<SipHandlerFn>>>,
}

impl Default for SipRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl SipRouter {
    /// 创建一个空的路由器
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(DashMap::new()),
            default_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// 注册一个 SIP 消息处理器
    ///
    /// - `method`: `Some("REGISTER")` 表示仅处理 REGISTER 方法，
    ///   `None` 表示作为 catch‑all（匹配所有未精确匹配的方法）。
    /// - `priority`: 优先级，数值越小越优先（同方法内有效）。
    /// - `handler`: 异步处理函数，签名为 `async fn(Transaction) -> RIE<()>`。
    pub fn add_handler<M, F, Fut>(&self, method: Option<M>, priority: i32, handler: F)
    where
        M: AsRef<str>,
        F: Fn(Transaction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RIE<()>> + Send + 'static,
    {
        let key = method
            .map(|m| m.as_ref().to_uppercase())
            .unwrap_or_default();

        let handler_fn: SipHandlerFn = Arc::new(move |tx| Box::pin(handler(tx)));

        // 获取或创建对应方法的条目列表
        let mut entries = self.handlers.entry(key.clone()).or_insert_with(Vec::new);

        // 按优先级升序插入
        let pos = entries
            .binary_search_by_key(&priority, |(p, _)| *p)
            .unwrap_or_else(|idx| idx);
        entries.insert(pos, (priority, handler_fn));

        info!(
            method = if key.is_empty() { "catch-all" } else { key.as_str() },
            priority,
            "SIP 处理器已注册 (total: {})",
            entries.len()
        );
    }

    /// 移除指定方法（及可选优先级）的处理器
    ///
    /// - `method`: 方法名，`None` 表示 catch‑all。
    /// - `priority`: 若为 `Some(p)`，只移除优先级等于 `p` 的处理器；
    ///   若为 `None`，移除该方法的全部处理器。
    ///
    /// 返回实际移除的数量。
    pub fn remove_handler(
        &self,
        method: Option<impl AsRef<str>>,
        priority: Option<i32>,
    ) -> usize {
        let key = method
            .map(|m| m.as_ref().to_uppercase())
            .unwrap_or_default();

        let Some(mut entries) = self.handlers.get_mut(&key) else {
            return 0;
        };

        let old_len = entries.len();
        match priority {
            Some(p) => {
                entries.retain(|(ep, _)| *ep != p);
            }
            None => {
                entries.clear();
            }
        }
        let removed = old_len - entries.len();

        // 若列表为空，清理键（避免内存泄漏）
        if entries.is_empty() {
            drop(entries); // 释放写锁
            self.handlers.remove(&key);
        }

        if removed > 0 {
            info!(
                method = if key.is_empty() { "catch-all" } else { key.as_str() },
                removed,
                "已移除 SIP 处理器"
            );
        }
        removed
    }

    /// 设置一个自定义的默认处理器，替代内置的 405 行为
    ///
    /// 当没有任何精确匹配和 catch‑all 处理器时，将调用此处理器。
    /// 若设为 `None`，则恢复默认行为（返回 405）。
    pub fn set_default_handler<F, Fut>(&self, handler: Option<F>)
    where
        F: Fn(Transaction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RIE<()>> + Send + 'static,
    {
        let new_handler = handler.map(|h| {
            let h: SipHandlerFn = Arc::new(move |tx| Box::pin(h(tx)));
            h
        });
        let mut default = self.default_handler.write().expect("default_handler lock");
        *default = new_handler;
    }

    /// 分发 SIP 消息到匹配的处理器
    ///
    /// 查找顺序：
    /// 1. 精确匹配请求方法名（O(1) 哈希 + 取列表中第一个条目）
    /// 2. catch‑all（键为空字符串，O(1)）
    /// 3. 用户自定义默认处理器（若有），否则内置 405
    pub async fn dispatch(&self, tx: Transaction) {
        let method = tx.original.method.to_string().to_uppercase();

        // 1. 精确匹配
        let handler = self
            .handlers
            .get(&method)
            .and_then(|entries| entries.first().map(|(_, h)| h.clone()))
            // 2. catch‑all
            .or_else(|| {
                self.handlers
                    .get("")
                    .and_then(|entries| entries.first().map(|(_, h)| h.clone()))
            });

        if let Some(h) = handler {
            if let Err(e) = h(tx).await {
                error!(method = %method, error = %e, "SIP 处理器执行出错");
            }
        } else {
            // 3. 用户自定义默认处理器 或 内置默认
            let default = self.default_handler.read().expect("default_handler lock").clone();
            if let Some(h) = default {
                if let Err(e) = h(tx).await {
                    error!(method = %method, error = %e, "自定义默认处理器执行出错");
                }
            } else {
                if let Err(e) = default_405_handler(tx).await {
                    error!(method = %method, error = %e, "405 默认处理器执行出错");
                }
            }
        }
    }

    /// 获取已注册的处理器总数（包括精确方法和 自定义 catch‑all）
    pub fn handler_count(&self) -> usize {
        self.handlers.iter().map(|kv| kv.value().len()).sum()
    }

    /// 返回所有注册的方法名（不含处理器细节）
    pub fn registered_methods(&self) -> Vec<String> {
        self.handlers
            .iter()
            .map(|kv| kv.key().clone())
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// 清空所有已注册的处理器和默认处理器
    pub fn clear(&self) {
        self.handlers.clear();
        let mut default = self.default_handler.write().expect("default_handler lock");
        *default = None;
        info!("已清空所有 SIP 处理器和默认处理器");
    }
}

// ---------- 内置兜底处理器 ----------
/// 返回 405 Method Not Allowed
async fn default_405_handler(mut tx: Transaction) -> RIE<()> {
    warn!(method = %tx.original.method, "未注册任何处理器，返回 405");
    tx.reply(StatusCode::MethodNotAllowed)
        .await
        .map_err(|e| anyhow::anyhow!("405 回复失败: {}", e))?;
    Ok(())
}