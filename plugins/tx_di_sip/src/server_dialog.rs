//! UAS INVITE 会话管理（服务端对话层）
//!
//! 补齐「平台/设备作为 UAS 接收 INVITE」的完整能力 —— 这是 GB28181
//! 语音广播/对讲（设备 → 平台 INVITE 推音频）建立的前提。
//!
//! 基于 rsipstack 已内置的 [`ServerInviteDialog`] 状态机，本模块负责：
//! - [`SipUasManager::on_invite`]：从入站事务创建会话（生成 to-tag、回 100 Trying）
//! - [`SipUasManager::accept`] / [`reject`]：业务应答（200 OK 携带 SDP answer / 拒绝）
//! - [`SipUasManager::on_bye`]：结束会话并清理（回复 200 + remove_dialog + 表清理）
//! - 状态转发：ACK → Confirmed、CANCEL → Terminated（经 DialogState 通道）
//!
//! 会话关联表复用 [`crate::dialog::InDialogTable`]，键为 [`DialogKey`]。

use rsipstack::dialog::dialog::{Dialog, DialogState, DialogStateReceiver, TerminatedReason};
use rsipstack::dialog::server_dialog::ServerInviteDialog;
use rsipstack::sip::{Method, SipMessage, StatusCode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};
use tx_di_core::{Component, DepsTuple, RIE};

use crate::dialog::{DialogKey, InDialogTable};
use crate::err::SipErr;
use crate::sip_tx::SipTx;
use crate::SipPlugin;

/// UAS 邀请会话：业务侧通过 [`SipUasManager`] 获取/控制
#[derive(Clone)]
pub struct UasSession {
    /// rsipstack 原生服务端对话框（accept/reject/bye 用）
    pub dialog: ServerInviteDialog,
    /// Call-ID
    pub call_id: String,
    /// 对端设备 ID（从 From 头提取）
    pub device_id: String,
    /// 请求 SDP（INVITE body，供业务解析 s= 类型）
    pub sdp_offer: Vec<u8>,
    /// 创建时刻
    pub created_at: Instant,
    /// 状态通知流（ACK → Confirmed / CANCEL → Terminated）
    state_rx: Arc<Mutex<Option<DialogStateReceiver>>>,
    /// 已清理标记
    removed: Arc<AtomicBool>,
}

impl UasSession {
    /// 取状态流（仅可消费一次）
    pub fn take_state_rx(&self) -> Option<DialogStateReceiver> {
        self.state_rx.lock().unwrap().take()
    }
}

/// UAS INVITE 管理器（DI 组件，init_sort 与 SipPlugin 同段）
///
/// 注入方式：
/// ```rust,ignore
/// #[component(...)]
/// struct X { uas: Arc<SipUasManager> }
/// ```
/// 在业务 `INVITE` handler 中调用 [`on_invite`](Self::on_invite) 创建会话，
/// 再调用 [`accept`](Self::accept) / [`reject`](Self::reject) 应答。
#[derive(Component)]
#[component(init_sort = 10000)]
pub struct SipUasManager {
    /// SIP 插件（提供 dialog_layer）
    pub sip: Arc<SipPlugin>,
    /// 会话关联表（DialogKey → UasSession）
    #[tx_cst(skip)]
    sessions: InDialogTable<UasSession>,
}

impl SipUasManager {
    /// 从入站 INVITE 事务创建 UAS 会话
    ///
    /// - 生成 to-tag 并注册 dialog 到 DialogLayer（后续 BYE/INFO 可 match）
    /// - 回 100 Trying
    /// - 启动后台驱动任务：ACK → 发 `DialogState::Confirmed`；CANCEL → 发 `Terminated`
    ///
    /// 业务侧拿到 [`UasSession`] 后调用 [`accept`](Self::accept) / [`reject`](Self::reject)。
    pub async fn on_invite(&self, tx: &SipTx, device_id: &str) -> RIE<UasSession> {
        let dialog_layer = self.sip.dialog_layer();
        let (state_tx, state_rx) = dialog_layer.new_dialog_state_channel();

        // 取出真实 Transaction（此后不可再通过 SipTx 回复，由 dialog 接管）
        let mut transaction = tx.take_transaction().await.ok_or(SipErr::TransactionMissing)?;

        // 半对话键 → 完整键（生成 to-tag 后）
        let half_key = DialogKey::from_request(&transaction.original).ok_or(SipErr::InvalidUri)?;

        let dialog = dialog_layer
            .get_or_create_server_invite(&transaction, state_tx.clone(), None, None)
            .map_err(|_| SipErr::UasInviteFailed)?;

        let call_id = dialog.id().call_id.clone();
        let full_key = half_key.with_to_tag(dialog.id().local_tag.clone());

        let session = UasSession {
            dialog: dialog.clone(),
            call_id: call_id.clone(),
            device_id: device_id.to_string(),
            sdp_offer: transaction.original.body.clone(),
            created_at: Instant::now(),
            state_rx: Arc::new(Mutex::new(Some(state_rx))),
            removed: Arc::new(AtomicBool::new(false)),
        };

        self.sessions.insert(full_key.clone(), session.clone());

        // 后台驱动：等 ACK → Confirmed；CANCEL → Terminated（清除会话）
        let sessions = self.sessions.clone();
        let dlg = dialog.clone();
        let key = full_key.clone();
        let stx = state_tx.clone();
        let call_id_spawn = call_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = transaction.receive().await {
                match msg {
                    SipMessage::Request(req) if req.method == Method::Ack => {
                        debug!(call_id = %call_id_spawn, "UAS 收到 ACK，会话确认");
                        let state = DialogState::Confirmed(
                            dlg.id(),
                            transaction.last_response.clone().unwrap_or_default(),
                        );
                        let _ = stx.send(state);
                        break;
                    }
                    SipMessage::Request(req) if req.method == Method::Cancel => {
                        info!(call_id = %call_id_spawn, "UAS 收到 CANCEL，会话终止");
                        let _ = stx.send(DialogState::Terminated(
                            dlg.id(),
                            TerminatedReason::UacCancel,
                        ));
                        sessions.remove(&key);
                        break;
                    }
                    _ => {}
                }
            }
            debug!(call_id = %call_id_spawn, "UAS 驱动任务退出");
        });

        info!(
            call_id = %call_id,
            device_id = %device_id,
            "📥 收到设备 INVITE，创建 UAS 会话"
        );
        Ok(session)
    }

    /// 接受 INVITE（200 OK + SDP answer）
    ///
    /// `public_contact`：NAT 场景传设备可访问的公网地址（回 Contact 头），
    /// 不传则用 endpoint 本地地址。
    pub fn accept(
        &self,
        session: &UasSession,
        sdp_answer: &[u8],
        public_contact: Option<rsipstack::sip::HostWithPort>,
    ) -> RIE<()> {
        match public_contact {
            Some(pub_addr) => {
                let local = self
                    .sip
                    .dialog_layer()
                    .endpoint
                    .transport_layer
                    .get_addrs()
                    .first()
                    .cloned();
                match local {
                    Some(local_addr) => session
                        .dialog
                        .accept_with_public_contact(
                            &session.device_id,
                            Some(pub_addr),
                            &local_addr,
                            None,
                            Some(sdp_answer.to_vec()),
                        )
                        .map_err(|_| SipErr::UasInviteFailed.into()),
                    None => session
                        .dialog
                        .accept(None, Some(sdp_answer.to_vec()))
                        .map_err(|_| SipErr::UasInviteFailed.into()),
                }
            }
            None => session
                .dialog
                .accept(None, Some(sdp_answer.to_vec()))
                .map_err(|_| SipErr::UasInviteFailed.into()),
        }
    }

    /// 拒绝 INVITE（默认 603 Decline，可指定状态码）
    pub fn reject(&self, session: &UasSession, code: Option<StatusCode>) -> RIE<()> {
        session
            .dialog
            .reject(code, None)
            .map_err(|_| SipErr::UasInviteFailed.into())
    }

    /// 处理 BYE（结束会话）：回复 200、清理 dialog 与关联表
    pub async fn on_bye(&self, tx: &SipTx) -> RIE<()> {
        let key = DialogKey::from_request(tx.request()).ok_or(SipErr::InvalidUri)?;

        if let Some(session) = self.sessions.lookup(&key) {
            let dialog_layer = self.sip.dialog_layer();
            let mut transaction = tx.take_transaction().await.ok_or(SipErr::TransactionMissing)?;

            // 让 dialog 状态机处理 BYE（内部回复 200 OK + 转 Terminated）
            let mut dlg = session.dialog.clone();
            if let Some(Dialog::ServerInvite(d)) = dialog_layer.match_dialog(&transaction) {
                dlg = d;
            }
            if let Err(e) = dlg.handle(&mut transaction).await {
                warn!(call_id = %session.call_id, error = %e, "UAS BYE 处理失败，回 200 兜底");
                tx.reply(StatusCode::OK)
                    .await
                    .map_err(|e| anyhow::anyhow!("回复 BYE 200 失败: {}", e))?;
            }

            dialog_layer.remove_dialog(&dlg.id());
            self.sessions.remove(&key);
            if !session.removed.swap(true, Ordering::SeqCst) {
                info!(call_id = %session.call_id, "📴 UAS 会话结束（BYE）");
            }
        } else {
            // 未知会话的 BYE：回 200 即可（设备可能挂断一个平台已清理的会话）
            tx.reply(StatusCode::OK)
                .await
                .map_err(|e| anyhow::anyhow!("回复 BYE 200 失败: {}", e))?;
        }
        Ok(())
    }

    /// 主动结束 UAS 会话（平台侧调用，如广播结束）：发 BYE + 清理
    pub async fn hangup(&self, session: &UasSession) -> RIE<()> {
        if let Err(e) = session.dialog.bye().await {
            warn!(call_id = %session.call_id, error = %e, "UAS 主动 BYE 失败（忽略）");
        }
        let key = DialogKey {
            call_id: session.call_id.clone(),
            from_tag: session.dialog.id().remote_tag.clone(),
            to_tag: Some(session.dialog.id().local_tag.clone()),
        };
        self.sessions.remove(&key);
        if !session.removed.swap(true, Ordering::SeqCst) {
            self.sip.dialog_layer().remove_dialog(&session.dialog.id());
        }
        Ok(())
    }

    /// 活跃 UAS 会话列表
    pub fn active_sessions(&self) -> Vec<UasSession> {
        self.sessions
            .iter_values()
            .into_iter()
            .map(|arc| (*arc).clone())
            .collect()
    }

    /// 按 Call-ID 查会话
    pub fn by_call_id(&self, call_id: &str) -> Option<UasSession> {
        self.sessions
            .iter_values()
            .into_iter()
            .find(|s| s.call_id == call_id)
            .map(|arc| (*arc).clone())
    }
}
