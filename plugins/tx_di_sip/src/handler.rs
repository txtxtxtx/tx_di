use dashmap::DashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::{error, info, warn};

use rsipstack::sip::StatusCode;
use rsipstack::transaction::transaction::Transaction;
use tx_di_core::{Component, DepsTuple, RIE};

use crate::middleware::{build_chain, SipMiddleware};
use crate::sip_tx::SipTx;

// ---------- 异步处理器类型 ----------
/// SIP 消息异步处理函数（线程安全、可共享）
///
/// 接收 [`SipTx`] 共享信封，返回 `RIE<()>`。
pub type SipHandlerFn = Arc<
    dyn Fn(SipTx) -> Pin<Box<dyn Future<Output = RIE<()>> + Send + 'static>>
    + Send
    + Sync,
>;

// ---------- 内部条目结构 ----------
/// 索引中存储的条目：优先级 + 处理器
type HandlerEntry = (i32, SipHandlerFn);

// ---------- SipRouter 定义 ----------
/// SIP 消息路由器（DI 组件）
///
/// 支持按 SIP 方法名注册异步处理器，并支持 catch‑all 兜底。
/// 内部使用 `DashMap` 实现高并发、无锁读的 O(1) 分发。
///
/// 作为 DI 组件注册后，由 `SipPlugin::app_async_init` 注入收集到的
/// 中间件（[`SipMiddleware`](crate::middleware::SipMiddleware)），
/// `dispatch` 在分发前让消息真正经过洋葱链。
///
/// # 特性
///
/// - 精确方法匹配（如 `"REGISTER"`, `"INVITE"`）和 catch‑all（`method: None`）
/// - 优先级排序：同方法内按 `priority` 升序执行
/// - 中间件洋葱链：`pre` 检视/短路 → handler → `post` 日志/指标
/// - 兜底 405：链结束仍无人回复时自动回复，防止 `Transaction` Drop 无响应
/// - 线程安全，可随意 Clone 并在多任务间共享（实际为单例注入）
#[derive(Component)]
#[component(init_sort = 10000)]
pub struct SipRouter {
    /// 方法名（大写）→ 按优先级排序的处理器列表
    /// 空字符串 `""` 作为 catch‑all 的键
    #[tx_cst(skip)]
    handlers: Arc<DashMap<String, Vec<HandlerEntry>>>,

    /// 可选的默认处理器（当没有任何处理器匹配时调用）
    #[tx_cst(skip)]
    default_handler: Arc<RwLock<Option<SipHandlerFn>>>,

    /// 注入的中间件链（由 SipPlugin 启动时填充）
    #[tx_cst(skip)]
    middlewares: Arc<RwLock<Vec<Arc<dyn SipMiddleware>>>>,
}

impl Default for SipRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl SipRouter {
    /// 创建一个空的路由器（仅在测试或非 DI 场景使用）
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(DashMap::new()),
            default_handler: Arc::new(RwLock::new(None)),
            middlewares: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注入中间件集合（由 `SipPlugin::app_async_init` 调用）
    pub fn set_middlewares(&self, mws: Vec<Arc<dyn SipMiddleware>>) {
        *self.middlewares.write().expect("middlewares lock") = mws;
    }

    /// 注册一个 SIP 消息处理器
    ///
    /// - `method`：`Some("REGISTER")` 表示仅处理 REGISTER 方法，
    ///   `None` 表示作为 catch‑all（匹配所有未精确匹配的方法）。
    /// - `priority`：优先级，数值越小越优先（同方法内有效）。
    /// - `handler`：异步处理函数，签名为 `async fn(SipTx) -> RIE<()>`。
    pub fn add_handler<F, Fut>(&self, method: Option<impl AsRef<str>>, priority: i32, handler: F)
    where
        F: Fn(SipTx) -> Fut + Send + Sync + 'static,
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
    /// - `method`：方法名，`None` 表示 catch‑all。
    /// - `priority`：若为 `Some(p)`，只移除优先级等于 `p` 的处理器；
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
        F: Fn(SipTx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RIE<()>> + Send + 'static,
    {
        let new_handler = handler.map(|h| {
            let h: SipHandlerFn = Arc::new(move |tx| Box::pin(h(tx)));
            h
        });
        let mut default = self.default_handler.write().expect("default_handler lock");
        *default = new_handler;
    }

    /// 分发 SIP 消息到匹配的处理器（并经过中间件洋葱链）
    ///
    /// 查找顺序：
    /// 1. 精确匹配请求方法名（O(1) 哈希 + 取列表中第一个条目）
    /// 2. catch‑all（键为空字符串，O(1)）
    /// 3. 用户自定义默认处理器（若有），否则内置 405
    ///
    /// 无论走哪条路径，都会先经过已注入的中间件链；链结束若仍无人回复，
    /// 自动回复 405，防止服务端 `Transaction` 因 Drop 而无响应。
    pub async fn dispatch(&self, tx: Transaction) {
        let sip_tx = SipTx::new(tx);
        let method = sip_tx.method().to_string().to_uppercase();

        // 1. 精确匹配 → 2. catch‑all
        let handler = self
            .handlers
            .get(&method)
            .and_then(|entries| entries.first().map(|(_, h)| h.clone()))
            .or_else(|| {
                self.handlers
                    .get("")
                    .and_then(|entries| entries.first().map(|(_, h)| h.clone()))
            });

        // 3. 用户自定义默认处理器 或 内置 405 兜底
        let handler: SipHandlerFn = match handler {
            Some(h) => h,
            None => {
                let default = self.default_handler.read().expect("default_handler lock").clone();
                match default {
                    Some(h) => h,
                    None => {
                        if !sip_tx.replied() {
                            if let Err(e) = sip_tx.reply(StatusCode::MethodNotAllowed).await {
                                error!(method = %method, error = %e, "405 兜底回复失败");
                            }
                        }
                        return;
                    }
                }
            }
        };

        // 经过中间件洋葱链（已注入 middlewares）
        let mws = self.middlewares.read().expect("middlewares lock").clone();
        let chain = build_chain(&mws, handler);
        if let Err(e) = chain(sip_tx.clone()).await {
            error!(method = %method, error = %e, "SIP 处理链执行出错");
        }

        // 兜底：链结束仍无人回复 → 405（防 Transaction Drop 无响应）
        if !sip_tx.replied() {
            if let Err(e) = sip_tx.reply(StatusCode::MethodNotAllowed).await {
                error!(method = %method, error = %e, "405 兜底回复失败");
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
/// 返回 405 Method Not Allowed（保留供显式调用场景）
#[allow(dead_code)]
async fn default_405_handler(sip_tx: SipTx) -> RIE<()> {
    warn!(method = %sip_tx.method(), "未注册任何处理器，返回 405");
    sip_tx
        .reply(StatusCode::MethodNotAllowed)
        .await
        .map_err(|e| anyhow::anyhow!("405 回复失败: {}", e))?;
    Ok(())
}
