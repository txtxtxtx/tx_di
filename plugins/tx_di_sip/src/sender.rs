//! SIP 消息发送接口
//!
//! 提供对上层友好的 SIP 请求发送 API，封装 rsipstack 的 DialogLayer 和 Registration。

use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::InviteOption;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip as rsip;
use rsipstack::transaction::endpoint::EndpointInnerRef;
use std::sync::Arc;
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
}

impl SipSender {
    pub(crate) fn new(endpoint: EndpointInnerRef) -> Self {
        Self { endpoint }
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
        .map_err(|e| anyhow::anyhow!("无效的注册服务器地址 '{}': {}", registrar, e))?;

        let credential = Credential {
            username: username.to_string(),
            password: password.to_string(),
            realm: None,
        };

        let mut reg = Registration::new(self.endpoint.clone(), Some(credential));
        let resp = reg
            .register(registrar_uri, None)
            .await
            .map_err(|e| anyhow::anyhow!("REGISTER 失败: {}", e))?;

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
            .map_err(|e| anyhow::anyhow!("无效的主叫 URI '{}': {}", caller, e))?;
        let callee_uri = rsip::Uri::try_from(callee)
            .map_err(|e| anyhow::anyhow!("无效的被叫 URI '{}': {}", callee, e))?;

        let dialog_layer = Arc::new(DialogLayer::new(self.endpoint.clone()));
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
            .map_err(|e| anyhow::anyhow!("INVITE 失败: {}", e))?;

        Ok((dialog, resp))
    }

    // ── 原始请求 ─────────────────────────────────────────────────────────────

    /// 获取底层 EndpointInnerRef，供高级用户直接操作 rsipstack API
    pub fn inner(&self) -> EndpointInnerRef {
        self.endpoint.clone()
    }

    /// 获取 DialogLayer（供高级用户使用）
    pub fn dialog_layer(&self) -> Arc<DialogLayer> {
        Arc::new(DialogLayer::new(self.endpoint.clone()))
    }
}
