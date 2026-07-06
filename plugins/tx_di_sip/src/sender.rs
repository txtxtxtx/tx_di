//! SIP 消息发送接口
//!
//! 提供对上层友好的 SIP 请求发送 API，封装 rsipstack 的 DialogLayer、Registration
//! 以及基于 `Transaction` 的 out-of-dialog 请求（MESSAGE / NOTIFY / SUBSCRIBE / INFO）。

use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::InviteOption;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip as rsip;
use rsipstack::sip::{CallId, ContentType, CSeq, Event, Expires, From, MaxForwards, To, Via};
use rsipstack::sip::StatusCodeKind;
use rsipstack::transaction::endpoint::EndpointInnerRef;
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::info;
use tx_di_core::RIE;

use crate::config::{SipConfig, SipTransport};
use crate::SipErr;

/// SIP 发送器
///
/// 持有 `EndpointInnerRef`，提供常用 SIP 操作的简洁 API。
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
    dialog_layer: OnceLock<Arc<DialogLayer>>,
}

impl SipSender {
    pub(crate) fn new(endpoint: EndpointInnerRef, config: Arc<SipConfig>) -> Self {
        Self {
            endpoint,
            config,
            dialog_layer: OnceLock::new(),
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

    // ── 呼叫 ────────────────────────────────────────────────────────────────

    /// 向目标发起 INVITE 呼叫
    pub async fn invite(
        &self,
        caller: &str,
        callee: &str,
        sdp_offer: Option<Vec<u8>>,
        credential: Option<Credential>,
    ) -> RIE<(
        rsipstack::dialog::client_dialog::ClientInviteDialog,
        Option<rsip::Response>,
    )> {
        let caller_uri = rsip::Uri::try_from(caller).map_err(|_| SipErr::InvalidUri)?;
        let callee_uri = rsip::Uri::try_from(callee).map_err(|_| SipErr::InvalidUri)?;

        let dialog_layer = self.dialog_layer();
        let (state_sender, _state_receiver) = dialog_layer.new_dialog_state_channel();

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: sdp_offer.as_ref().map(|_| "application/sdp".to_string()),
            offer: sdp_offer.map(|b| b.into()),
            credential,
            ..Default::default()
        };

        let (dialog, resp) = dialog_layer
            .do_invite(invite_option, state_sender)
            .await
            .map_err(|_| SipErr::InviteFailed)?;

        Ok((dialog, resp))
    }

    // ── 原始请求 ─────────────────────────────────────────────────────────────

    /// 获取底层 EndpointInnerRef，供高级用户直接操作 rsipstack API
    pub fn inner(&self) -> EndpointInnerRef {
        self.endpoint.clone()
    }

    /// 获取 DialogLayer（供高级用户使用）
    pub fn dialog_layer(&self) -> Arc<DialogLayer> {
        self.dialog_layer
            .get_or_init(|| Arc::new(DialogLayer::new(self.endpoint.clone())))
            .clone()
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
        tx.send()
            .await
            .map_err(|e| anyhow::anyhow!("SIP {} 发送失败: {}", method, e))?;

        // 等待最终响应（跳过 1xx 临时响应）
        while let Some(msg) = tx.receive().await {
            if let rsip::SipMessage::Response(resp) = msg {
                if resp.status_code.kind() != StatusCodeKind::Provisional {
                    return Ok(resp);
                }
            }
        }
        Err(SipErr::MessageFailed.into())
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
