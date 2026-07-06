//! SIP 事务共享信封
//!
//! rsipstack 的 [`Transaction`] 类型 **`!Clone`**，且所有回复方法
//! （`reply` / `reply_with` / `respond` / `send`）均为 `&mut self` + `async`。
//! 因此中间件链**无法按值传递 `Transaction` 并在 post-step 访问它**，
//! 也无法通过 clone 重试/审计。
//!
//! [`SipTx`] 是一个薄信封：
//! - 生产环境内部持有真实的 `Transaction`（`Arc<Mutex<Option<Transaction>>>`）
//! - 构造时缓存 `Request` 克隆，只读检视（方法名/头/body）零锁
//! - 回复具备**幂等**语义（首个回复真正发送，之后的被忽略），防止多个
//!   中间件或兜底逻辑重复回复
//! - 提供 `fake()` 测试构造器：无需 `Endpoint` 即可构造，回复仅记录状态码，
//!   使 handler / 中间件可在纯内存环境单测
//!
//! 设计目标：强绑定 rsipstack（内部即真实 `Transaction`），同时可共享、可测试。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rsipstack::sip::Header;
use rsipstack::sip::Request;
use rsipstack::sip::StatusCode;
use rsipstack::transaction::transaction::Transaction;
use tokio::sync::Mutex;
use tx_di_core::RIE;

use crate::SipErr;

/// 共享 SIP 事务信封
#[derive(Clone)]
pub struct SipTx {
    /// 真实事务（生产环境）；测试桩模式为 `None`
    inner: Arc<Mutex<Option<Transaction>>>,
    /// 构造时缓存的请求副本，只读检视零锁
    request: Request,
    /// 幂等回复标志（跨 `SipTx` 克隆共享）
    replied: Arc<AtomicBool>,
    /// 测试桩模式下的回复记录器（生产环境为 `None`）
    recorded: Option<Arc<Mutex<Option<StatusCode>>>>,
}

impl SipTx {
    /// 用真实事务构造（生产环境入站分发路径）
    pub fn new(tx: Transaction) -> Self {
        let request = tx.original.clone();
        Self {
            inner: Arc::new(Mutex::new(Some(tx))),
            request,
            replied: Arc::new(AtomicBool::new(false)),
            recorded: None,
        }
    }

    /// 构造测试桩（无需 `Endpoint`）
    ///
    /// 返回的 `(SipTx, recorder)` 中，`recorder` 在调用 `reply` 后记录状态码，
    /// 供断言使用。
    pub fn fake(request: Request) -> (Self, Arc<Mutex<Option<StatusCode>>>) {
        let recorded = Arc::new(Mutex::new(None));
        let tx = Self {
            inner: Arc::new(Mutex::new(None)),
            request,
            replied: Arc::new(AtomicBool::new(false)),
            recorded: Some(recorded.clone()),
        };
        (tx, recorded)
    }

    /// 只读：SIP 方法
    pub fn method(&self) -> rsipstack::sip::Method {
        self.request.method.clone()
    }

    /// 只读：原始请求（`Request` 可 Clone，访问构造时缓存的副本，零锁）
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// 是否已回复（幂等标志）
    pub fn replied(&self) -> bool {
        self.replied.load(Ordering::SeqCst)
    }

    /// 幂等回复：第一个回复真正发送，之后的被忽略（防重复回复）
    pub async fn reply(&self, code: StatusCode) -> RIE<()> {
        if self.replied.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(rec) = &self.recorded {
            *rec.lock().await = Some(code);
            return Ok(());
        }
        let mut g = self.inner.lock().await;
        let tx = g.as_mut().ok_or(SipErr::TransactionMissing)?;
        tx.reply(code)
            .await
            .map_err(|e| anyhow::anyhow!("SIP 回复失败: {}", e))?;
        Ok(())
    }

    /// 幂等回复（带额外头与 body）
    pub async fn reply_with(
        &self,
        code: StatusCode,
        headers: Vec<Header>,
        body: Option<Vec<u8>>,
    ) -> RIE<()> {
        if self.replied.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if self.recorded.is_some() {
            // 测试桩模式不区分 reply_with，仅记录状态码
            let rec = self.recorded.as_ref().unwrap();
            *rec.lock().await = Some(code);
            return Ok(());
        }
        let mut g = self.inner.lock().await;
        let tx = g.as_mut().ok_or(SipErr::TransactionMissing)?;
        tx.reply_with(code, headers, body)
            .await
            .map_err(|e| anyhow::anyhow!("SIP 回复失败: {}", e))?;
        Ok(())
    }

    /// 透传真实 `Transaction`（需要完整 API 时使用，如 in-dialog 请求）
    pub async fn with_transaction<R>(&self, f: impl FnOnce(&mut Transaction) -> R) -> R {
        let mut g = self.inner.lock().await;
        f(g.as_mut().expect("真实 Transaction 已被取出（仅测试桩模式下发生）"))
    }

    /// 取出真实 `Transaction`
    ///
    /// 仅在确定不再需要通过 `SipTx` 回复时调用（例如交给底层对话框处理）。
    /// 取出后 `replied()` 不再反映真实回复状态，调用方需自行保证只回复一次。
    pub async fn take_transaction(&self) -> Option<Transaction> {
        let mut g = self.inner.lock().await;
        g.take()
    }
}
