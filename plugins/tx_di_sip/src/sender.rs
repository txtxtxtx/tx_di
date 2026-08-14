//! SIP 消息发送接口
//!
//! 提供对上层友好的 SIP 请求发送 API，封装 rsipstack 的 DialogLayer、Registration
//! 以及基于 `Transaction` 的 out-of-dialog 请求（MESSAGE / NOTIFY / SUBSCRIBE / INFO）。

use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::client_dialog::ClientInviteDialog;
use rsipstack::dialog::dialog::{DialogState, DialogStateReceiver};
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::{InviteAsyncResult, InviteOption};
use rsipstack::dialog::registration::Registration;
use rsipstack::sip as rsip;
use rsipstack::sip::StatusCodeKind;
use rsipstack::sip::{CSeq, CallId, ContentType, Event, Expires, From, MaxForwards, To, Via};
use rsipstack::transaction::endpoint::EndpointInnerRef;
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tracing::info;
use tx_di_core::RIE;

use crate::SipErr;
use crate::config::{SipConfig, SipTransport};

/// INVITE 会话句柄：对话框 + 状态通知通道 + 生命周期守卫
///
/// 由 [`SipSender::invite`] 返回，持有：
/// - `dialog`：rsipstack 原生对话框（BYE/CANCEL/reINVITE 用）
/// - `state_rx`：`DialogState` 通知流（Confirmed / Terminated 等），可 [`take_state_rx`] 消费一次
/// - `cleanup()`：从 DialogLayer 移除 dialog（防内存泄漏）
///
/// **契约**：`do_invite` 成功后 dialog 会注册进 DialogLayer，
/// 业务方必须在会话结束（收到 `DialogState::Terminated` 或主动挂断）后调用
/// [`cleanup`](Self::cleanup)，否则泄漏。
#[derive(Clone)]
pub struct InviteHandle {
    pub dialog: ClientInviteDialog,
    pub call_id: String,
    /// 最终响应（2xx 的 SDP 解析用；未 2xx 或超时可能为 None）
    pub final_response: Option<rsip::Response>,
    state_rx: Arc<Mutex<Option<DialogStateReceiver>>>,
    dialog_layer: Arc<DialogLayer>,
    removed: Arc<AtomicBool>,
}

impl InviteHandle {
    /// 取状态流（仅可消费一次；不需要状态通知时可忽略）
    pub fn take_state_rx(&self) -> Option<DialogStateReceiver> {
        self.state_rx.try_lock().ok()?.take()
    }

    /// 从 DialogLayer 移除 dialog（幂等）
    pub fn cleanup(&self) {
        if !self.removed.swap(true, Ordering::SeqCst) {
            self.dialog_layer.remove_dialog(&self.dialog.id());
        }
    }

    /// 当前是否已清理
    pub fn cleaned(&self) -> bool {
        self.removed.load(Ordering::SeqCst)
    }

    /// 等待会话终止（监听状态流直到 Terminated），自动清理
    ///
    /// 适合「发起后不需要中间状态」的场景（如抓拍）。
    pub async fn wait_terminated(self) {
        if let Some(mut rx) = self.take_state_rx() {
            while let Some(state) = rx.recv().await {
                if matches!(state, DialogState::Terminated(..)) {
                    break;
                }
            }
        }
        self.cleanup();
    }
}

/// SIP 发送器
///
/// 持有 `EndpointInnerRef` + 共享 `DialogLayer`，提供常用 SIP 操作的简洁 API。
///
/// 通过 `SipPlugin::sender()` 获取实例：
///
/// ```rust,ignore
/// use tx_di_sip::SipPlugin;
///
/// // 在 async_init 中通过 App 获取
/// let sip = /* ctx.inject::<SipPlugin>() */ unreachable!();
/// let sender = sip.sender().unwrap();
/// sender.register("sip:registrar.example.com", "alice", "secret").await.unwrap();
/// ```
#[derive(Clone)]
pub struct SipSender {
    endpoint: EndpointInnerRef,
    config: Arc<SipConfig>,
    dialog_layer: Arc<DialogLayer>,
}

impl SipSender {
    pub(crate) fn new(
        endpoint: EndpointInnerRef,
        config: Arc<SipConfig>,
        dialog_layer: Arc<DialogLayer>,
    ) -> Self {
        Self {
            endpoint,
            config,
            dialog_layer,
        }
    }

    // ── 注册 ────────────────────────────────────────────────────────────────

    /// 向 SIP 注册服务器发起 REGISTER 注册
    pub async fn register(
        &self,
        registrar: &str,
        username: &str,
        password: &str,
    ) -> RIE<rsip::Response> {
        let registrar_uri = rsip::Uri::try_from(
            if registrar.starts_with("sip:") || registrar.starts_with("sips:") {
                registrar.to_string()
            } else {
                format!("sip:{}", registrar)
            }
            .as_str(),
        )
        .map_err(|_| SipErr::InvalidUri)?;

        let credential = Credential {
            username: username.to_string(),
            password: password.to_string(),
            realm: self.config.realm.clone(),
        };

        let mut reg = Registration::new(self.endpoint.clone(), Some(credential));
        let resp = reg
            .register(registrar_uri, None)
            .await
            .map_err(|_| SipErr::RegisterFailed)?;

        info!(status = %resp.status_code, "REGISTER 响应");
        Ok(resp)
    }

    /// 向上级 SIP 服务器发起注销（REGISTER with Expires: 0）
    ///
    /// 复用 rsipstack `Registration` 并携带 `Credential`，自动处理 401 重认证。
    pub async fn unregister(
        &self,
        registrar: &str,
        username: &str,
        password: &str,
    ) -> RIE<rsip::Response> {
        let registrar_uri = rsip::Uri::try_from(
            if registrar.starts_with("sip:") || registrar.starts_with("sips:") {
                registrar.to_string()
            } else {
                format!("sip:{}", registrar)
            }
            .as_str(),
        )
        .map_err(|_| SipErr::InvalidUri)?;

        let credential = Credential {
            username: username.to_string(),
            password: password.to_string(),
            realm: self.config.realm.clone(),
        };

        let mut reg = Registration::new(self.endpoint.clone(), Some(credential));
        let resp = reg
            .register(registrar_uri, Some(0))
            .await
            .map_err(|_| SipErr::RegisterFailed)?;

        info!(status = %resp.status_code, "UNREGISTER 响应");
        Ok(resp)
    }

    // ── 呼叫 ────────────────────────────────────────────────────────────────

    /// 向目标发起 INVITE 呼叫（返回会话句柄，含状态通知流）
    pub async fn invite(
        &self,
        caller: &str,
        callee: &str,
        sdp_offer: Option<Vec<u8>>,
        credential: Option<Credential>,
    ) -> RIE<InviteHandle> {
        let caller_uri = rsip::Uri::try_from(caller).map_err(|_| SipErr::InvalidUri)?;
        let callee_uri = rsip::Uri::try_from(callee).map_err(|_| SipErr::InvalidUri)?;

        let (state_sender, state_rx) = self.dialog_layer.new_dialog_state_channel();

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: sdp_offer.as_ref().map(|_| "application/sdp".to_string()),
            offer: sdp_offer.map(|b| b.into()),
            credential,
            ..Default::default()
        };

        let (dialog, resp) = self
            .dialog_layer
            .do_invite(invite_option, state_sender)
            .await
            .map_err(|_| SipErr::InviteFailed)?;

        let call_id = dialog.id().call_id.clone();
        Ok(InviteHandle {
            dialog,
            call_id,
            final_response: resp,
            state_rx: Arc::new(Mutex::new(Some(state_rx))),
            dialog_layer: self.dialog_layer.clone(),
            removed: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 后台异步 INVITE（不阻塞调用方；GB 点播/回放推荐）
    ///
    /// 返回 `(InviteHandle, JoinHandle)`；JoinHandle 完成后可通过 `resp` 判断
    /// 2xx 或失败。确认后的 dialog 生命周期同样由 InviteHandle 管理。
    pub fn invite_async(
        &self,
        caller: &str,
        callee: &str,
        sdp_offer: Option<Vec<u8>>,
        credential: Option<Credential>,
    ) -> RIE<(InviteHandle, tokio::task::JoinHandle<InviteAsyncResult>)> {
        let caller_uri = rsip::Uri::try_from(caller).map_err(|_| SipErr::InvalidUri)?;
        let callee_uri = rsip::Uri::try_from(callee).map_err(|_| SipErr::InvalidUri)?;

        let (state_sender, state_rx) = self.dialog_layer.new_dialog_state_channel();

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: sdp_offer.as_ref().map(|_| "application/sdp".to_string()),
            offer: sdp_offer.map(|b| b.into()),
            credential,
            ..Default::default()
        };

        let dl = self.dialog_layer.clone();
        let (dialog, handle) = dl
            .do_invite_async(invite_option, state_sender)
            .map_err(|_| SipErr::InviteFailed)?;

        let call_id = dialog.id().call_id.clone();
        let ih = InviteHandle {
            dialog,
            call_id,
            // do_invite_async 无同步响应；最终响应由 JoinHandle<InviteAsyncResult> 提供
            final_response: None,
            state_rx: Arc::new(Mutex::new(Some(state_rx))),
            dialog_layer: self.dialog_layer.clone(),
            removed: Arc::new(AtomicBool::new(false)),
        };
        Ok((ih, handle))
    }

    // ── 原始请求 ─────────────────────────────────────────────────────────────

    /// 获取底层 EndpointInnerRef，供高级用户直接操作 rsipstack API
    pub fn inner(&self) -> EndpointInnerRef {
        self.endpoint.clone()
    }

    /// 获取 DialogLayer（供高级用户使用；单例共享）
    pub fn dialog_layer(&self) -> Arc<DialogLayer> {
        self.dialog_layer.clone()
    }

    // ── 会话控制 ─────────────────────────────────────────────────────────────

    /// 发送 BYE 挂断呼叫
    pub async fn bye(
        &self,
        dialog: &rsipstack::dialog::client_dialog::ClientInviteDialog,
    ) -> RIE<()> {
        dialog.bye().await.map_err(|_| SipErr::ByeFailed)?;
        Ok(())
    }

    /// 发送 CANCEL 取消正在进行的 INVITE
    pub async fn cancel(
        &self,
        dialog: &rsipstack::dialog::client_dialog::ClientInviteDialog,
    ) -> RIE<()> {
        dialog.cancel().await.map_err(|_| SipErr::CancelFailed)?;
        Ok(())
    }

    // ── out-of-dialog 请求（MESSAGE / NOTIFY / SUBSCRIBE / INFO）─────────────

    /// 发送 MESSAGE（国标级联核心能力）
    pub async fn send_message(
        &self,
        to: &str,
        from: &str,
        body: &[u8],
        content_type: &str,
    ) -> RIE<rsip::Response> {
        self.send_out_of_dialog(
            rsip::Method::Message,
            to,
            from,
            Some(body.to_vec()),
            Some(content_type),
            vec![],
        )
        .await
    }

    /// 发送 NOTIFY
    pub async fn notify(
        &self,
        to: &str,
        from: &str,
        body: &[u8],
        sub_state: &str,
    ) -> RIE<rsip::Response> {
        let extra = vec![rsip::Header::Event(Event::new(sub_state))];
        self.send_out_of_dialog(
            rsip::Method::Notify,
            to,
            from,
            Some(body.to_vec()),
            Some("application/msg+sip"),
            extra,
        )
        .await
    }

    /// 发送 SUBSCRIBE
    pub async fn subscribe(
        &self,
        to: &str,
        from: &str,
        event: &str,
        expires: u32,
    ) -> RIE<rsip::Response> {
        let extra = vec![
            rsip::Header::Event(Event::new(event)),
            rsip::Header::Expires(Expires::from(expires)),
        ];
        self.send_out_of_dialog(rsip::Method::Subscribe, to, from, None, None, extra)
            .await
    }

    /// 发送 INFO
    pub async fn info(&self, to: &str, from: &str, body: &[u8]) -> RIE<rsip::Response> {
        self.send_out_of_dialog(
            rsip::Method::Info,
            to,
            from,
            Some(body.to_vec()),
            Some("application/sdp"),
            vec![],
        )
        .await
    }

    /// 统一的 out-of-dialog 请求发送实现
    ///
    /// 构造完整 `Request`（Via/From/To/CallId/CSeq/MaxForwards + 可选 Content-Type），
    /// 通过 `Transaction::new_client` + `send()` 发送，并等待最终（非 1xx）响应。
    async fn send_out_of_dialog(
        &self,
        method: rsip::Method,
        to: &str,
        from: &str,
        body: Option<Vec<u8>>,
        content_type: Option<&str>,
        extra: Vec<rsip::Header>,
    ) -> RIE<rsip::Response> {
        let to_uri = rsip::Uri::try_from(to).map_err(|_| SipErr::InvalidUri)?;
        let from_uri = rsip::Uri::try_from(from).map_err(|_| SipErr::InvalidUri)?;

        let transport_str = match self.config.transport {
            SipTransport::Tcp | SipTransport::Tls => "TCP",
            SipTransport::Ws => "WS",
            _ => "UDP",
        };
        let contact = self.config.contact_ip();
        let via = Via::new(format!(
            "SIP/2.0/{} {};branch={}",
            transport_str,
            contact,
            make_branch()
        ));
        let from_hdr = From::new(format!("<{}>;tag={}", from_uri, make_tag()));
        let to_hdr = To::new(format!("<{}>", to_uri));
        let call_id = CallId::new(make_call_id());
        let cseq = CSeq::new(format!("{} {}", 1u32, method));

        let mut headers: Vec<rsip::Header> = vec![
            via.into(),
            call_id.into(),
            from_hdr.into(),
            to_hdr.into(),
            cseq.into(),
            MaxForwards::new("70").into(),
        ];
        if let Some(ct) = content_type {
            headers.push(rsip::Header::ContentType(ContentType::new(ct)));
        }
        headers.extend(extra);
        let body = body.unwrap_or_default();

        let request = rsip::Request {
            method,
            uri: to_uri,
            headers: headers.into(),
            body,
            version: rsip::Version::V2,
        };

        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("事务键生成失败: {}", e))?;
        let mut tx = Transaction::new_client(key, request, self.endpoint.clone(), None);
        let method_str = method.to_string();

        // 整个发送+等待过程受应用层超时保护（默认 5s，见 outbound_timeout_secs）
        let timeout = self.config.outbound_timeout();
        tokio::time::timeout(timeout, async {
            // 注意：tx.send() 失败不返回 Err（事务层进入 Calling 由 Timer A 重试），
            // 因此这里只需真正等待最终响应
            if let Err(e) = tx.send().await {
                tracing::warn!(method = %method_str, error = %e, "SIP 请求发送失败（事务层将重试）");
            }

            // 等待最终响应（跳过 1xx 临时响应）
            while let Some(msg) = tx.receive().await {
                if let rsip::SipMessage::Response(resp) = msg {
                    if resp.status_code.kind() != StatusCodeKind::Provisional {
                        return Ok(resp);
                    }
                }
            }
            Err(SipErr::MessageFailed.into())
        })
        .await
        .map_err(|_elapsed| SipErr::RequestTimeout)?
    }

    /// 向目标发送 OPTIONS 探活，返回是否收到 200
    pub async fn ping(&self, to: &str) -> RIE<bool> {
        let from = format!("sip:{}", self.config.contact_ip());
        let resp = self
            .send_out_of_dialog(rsip::Method::Options, to, &from, None, None, vec![])
            .await?;
        Ok(resp.status_code == rsip::StatusCode::OK)
    }
}

/// 生成 Via branch 参数（RFC 3261：z9hG4bK + 随机串）
fn make_branch() -> String {
    format!("z9hG4bK{:016X}", rand::random::<u64>())
}

/// 生成 From/To tag
fn make_tag() -> String {
    format!("{:08X}", rand::random::<u32>())
}

/// 生成 Call-ID
fn make_call_id() -> String {
    format!("{:022X}", rand::random::<u128>())
}
