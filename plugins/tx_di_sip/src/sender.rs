//! SIP 消息发送接口
//!
//! 提供对上层友好的 SIP 请求发送 API，封装 rsipstack 的 DialogLayer 和 Registration。

use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::InviteOption;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip as rsip;
use rsipstack::transaction::endpoint::EndpointInnerRef;
use crate::config::SipConfig;
use crate::SipErr;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::info;

/// SIP 发送器
///
/// 持有 `EndpointInnerRef`，提供常用 SIP 操作的简洁 API。
///
/// 通过 `SipPlugin::sender()` 获取实例：
///
/// ```rust,no_run
/// use tx_di_sip::SipPlugin;
/// use std::sync::Arc;
///
/// // 在 async_init 中通过 App 获取
/// let sip = ctx.inject::<SipPlugin>();
/// let sender = sip.sender();
/// sender.register("sip:registrar.example.com", "alice", "secret").await?;
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
    ///
    /// # 参数
    ///
    /// - `registrar` — 注册服务器 URI，例如 `"sip:registrar.example.com"`
    /// - `username` — SIP 用户名
    /// - `password` — SIP 密码（会自动处理 401/407 摘要认证）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// sender.register("sip:10.0.0.1:5060", "1001", "pass123").await?;
    /// ```
    pub async fn register(
        &self,
        registrar: &str,
        username: &str,
        password: &str,
    ) -> anyhow::Result<rsip::Response> {
        let registrar_uri = rsip::Uri::try_from(
            if registrar.starts_with("sip:") || registrar.starts_with("sips:") {
                registrar.to_string()
            } else {
                format!("sip:{}", registrar)
            }
            .as_str(),
        )
        .map_err(|_| SipErr::InvalidUri)?;

        // 认证信息
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
    ///
    /// # 参数
    ///
    /// - `caller` — 主叫 SIP URI，例如 `"sip:alice@192.168.1.2:5060"`
    /// - `callee` — 被叫 SIP URI，例如 `"sip:bob@192.168.1.3"`
    /// - `sdp_offer` — 可选的 SDP offer 体（`application/sdp`）
    /// - `credential` — 可选的认证凭据（呼叫需要认证时传入）
    ///
    /// # 返回
    ///
    /// 返回 `(dialog_layer, response)`，调用者可通过 dialog 继续操作（发送 BYE 等）。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// let (dialog, resp) = sender
    ///     .invite("sip:alice@192.168.1.100:5060", "sip:bob@192.168.1.200", None, None)
    ///     .await?;
    /// if let Some(r) = resp {
    ///     println!("呼叫响应: {}", r.status_code);
    /// }
    /// ```
    pub async fn invite(
        &self,
        caller: &str,
        callee: &str,
        sdp_offer: Option<Vec<u8>>,
        credential: Option<Credential>,
    ) -> anyhow::Result<(
        rsipstack::dialog::client_dialog::ClientInviteDialog,
        Option<rsip::Response>,
    )> {
        let caller_uri = rsip::Uri::try_from(caller)
            .map_err(|_| SipErr::InvalidUri)?;
        let callee_uri = rsip::Uri::try_from(callee)
            .map_err(|_| SipErr::InvalidUri)?;

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
    ///
    /// 首次调用时创建并缓存，后续调用直接返回缓存实例。
    pub fn dialog_layer(&self) -> Arc<DialogLayer> {
        self.dialog_layer
            .get_or_init(|| Arc::new(DialogLayer::new(self.endpoint.clone())))
            .clone()
    }

    // ── 会话控制 ─────────────────────────────────────────────────────────────

    /// 发送 BYE 挂断呼叫
    ///
    /// 在 INVITE 建立的对话上发送 BYE 请求，正常终止会话。
    ///
    /// # 参数
    ///
    /// - `dialog` — INVITE 建立的客户端对话
    pub async fn bye(
        &self,
        dialog: &rsipstack::dialog::client_dialog::ClientInviteDialog,
    ) -> anyhow::Result<()> {
        dialog
            .bye()
            .await
            .map_err(|e| anyhow::anyhow!(SipErr::ByeFailed).context(e))
    }

    /// 发送 CANCEL 取消正在进行的 INVITE
    ///
    /// 在 INVITE 尚未被接听时发送 CANCEL 请求。
    ///
    /// # 参数
    ///
    /// - `dialog` — INVITE 建立的客户端对话
    pub async fn cancel(
        &self,
        dialog: &rsipstack::dialog::client_dialog::ClientInviteDialog,
    ) -> anyhow::Result<()> {
        dialog
            .cancel()
            .await
            .map_err(|e| anyhow::anyhow!(SipErr::CancelFailed).context(e))
    }

    // ── 消息发送 ─────────────────────────────────────────────────────────────

    // NOTE: MESSAGE 方法当前依赖 rsipstack 私有 Transaction API，
    // 等待 rsipstack 未来版本暴露高层 MESSAGE 发送接口后再实现。
    // 参见：https://github.com/rsipstack/rsipstack/issues
}
