//! GB28181 SIP 摘要认证中间件
//!
//! 作为 [`SipMiddleware`](tx_di_sip::SipMiddleware) 组件（DI 收集、洋葱链前置）拦截
//! 入站 REGISTER 请求，实现 GB28181-2022 §5.2 摘要认证与 ACL 前置校验：
//!
//! - **ACL 白/黑名单**：命中黑名单或不在白名单 → 403（最前置，最先拦截）
//! - **无 Authorization** → 401 质询（下发 `WWW-Authenticate: Digest ...`）
//! - **有 Authorization** → 校验 response；失败 403，成功放行
//!
//! 非 REGISTER 方法（MESSAGE / INVITE 等）直接放行，交由后续业务 handler 处理。
//! nonce 存储随中间件单例常驻，认证成功后即清除。

use dashmap::DashMap;
use std::sync::Arc;

use rsipstack::sip::{Header, HeadersExt, StatusCode};
use tracing::{debug, warn};
use tx_di_core::{Component, DepsTuple, RIE};
use tx_di_sip::{SipMiddleware, SipNextFn, SipTx};
use tx_gb28181::sip::extract_user_from_sip_uri;

use crate::config::Gb28181ServerConfig;
use tx_di_sip::auth::{generate_nonce, verify_digest_auth};

/// Nonce 存储：记录已发给每个设备的 nonce，防止重放攻击
///
/// 原定义于 `handlers.rs`，因认证逻辑整体迁移至本中间件，故一并迁入。
#[derive(Clone)]
pub struct NonceStore {
    inner: Arc<DashMap<String, String>>,
}

impl NonceStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// 生成并存储 nonce（使用加密安全随机数）
    pub fn issue(&self, device_id: &str) -> String {
        let nonce = generate_nonce();
        self.inner.insert(device_id.to_string(), nonce.clone());
        nonce
    }

    /// 读取已下发的 nonce（用于校验 `Authorization.response`）
    pub fn get(&self, device_id: &str) -> Option<String> {
        self.inner.get(device_id).map(|v| v.clone())
    }

    /// 删除 nonce（认证成功后清除）
    pub fn remove(&self, device_id: &str) {
        self.inner.remove(device_id);
    }
}

/// GB28181 摘要认证中间件
///
/// 通过 `#[component(as_trait = dyn SipMiddleware)]` 注册，由 `SipPlugin` 在启动时
/// DI 收集并注入 `SipRouter` 洋葱链最外层（[`sort`] 返回极小值）。
#[derive(Component)]
#[component(as_trait = dyn SipMiddleware)]
pub struct Gb28181AuthMiddleware {
    /// 配置（含 realm / enable_auth / 密码 / ACL 名单）
    pub config: Arc<Gb28181ServerConfig>,
    /// 摘要认证 nonce 存储
    #[tx_cst(NonceStore::new())]
    pub nonce_store: NonceStore,
}

#[async_trait::async_trait]
impl SipMiddleware for Gb28181AuthMiddleware {
    /// 认证层应最外层前置（先于业务 handler，数值越小越外层）
    fn sort(&self) -> i32 {
        10
    }

    fn name(&self) -> &str {
        "gb28181-auth"
    }

    async fn process(&self, tx: SipTx, next: SipNextFn) -> RIE<()> {
        // 仅对 REGISTER 做认证/ACL；其余方法直接放行
        if !tx.request().method.to_string().eq_ignore_ascii_case("REGISTER") {
            return next(tx).await;
        }

        let from_str = tx
            .request()
            .from_header()
            .map(|h| h.value().to_string())
            .unwrap_or_default();
        let device_id =
            extract_user_from_sip_uri(&from_str).unwrap_or_else(|| from_str.clone());

        // ── ACL 前置校验（黑名单/白名单）────────────────────────────────
        if let Err(reason) = self.config.check_device_allowed(&device_id) {
            warn!(device_id = %device_id, reason = %reason, "🚫 ACL 拒绝注册");
            tx.reply_with(
                StatusCode::Forbidden,
                vec![],
                Some(reason.as_bytes().to_vec()),
            )
            .await
            .map_err(|e| anyhow::anyhow!("发送 403 Forbidden 失败: {}", e))?;
            return Ok(());
        }

        // 解析 Expires：注销（expires=0）无需认证
        let expires = tx
            .request()
            .expires_header()
            .map(|h| h.value().to_string().parse::<u32>().unwrap_or(3600))
            .unwrap_or(3600);

        if !self.config.enable_auth || expires == 0 {
            return next(tx).await;
        }

        // ── 摘要认证 ─────────────────────────────────────────────────────
        let auth_header = tx
            .request()
            .headers
            .iter()
            .find(|h| format!("{}", h).to_lowercase().starts_with("authorization:"))
            .map(|h| format!("{}", h));

        match auth_header {
            None => {
                // 无 Authorization → 401 质询
                let nonce = self.nonce_store.issue(&device_id);
                let www_auth = format!(
                    "Digest realm=\"{}\", nonce=\"{}\", algorithm=MD5",
                    self.config.realm, nonce
                );
                let headers = vec![Header::WwwAuthenticate(
                    rsipstack::sip::WwwAuthenticate::new(www_auth),
                )];
                tx.reply_with(StatusCode::Unauthorized, headers, None)
                    .await
                    .map_err(|e| anyhow::anyhow!("发送 401 Unauthorized 失败: {}", e))?;
                debug!(device_id = %device_id, "🔐 发送 401 质询");
                Ok(())
            }
            Some(auth) => {
                // 有 Authorization → 校验
                let nonce = self.nonce_store.get(&device_id).unwrap_or_default();
                let req_uri = tx.request().uri.to_string();

                let ok = verify_digest_auth(
                    &auth,
                    "REGISTER",
                    &req_uri,
                    self.config.get_password(&device_id),
                    &self.config.realm,
                    &nonce,
                );

                if !ok {
                    warn!(device_id = %device_id, "🔐 摘要认证失败，拒绝注册");
                    tx.reply(StatusCode::Forbidden)
                        .await
                        .map_err(|e| anyhow::anyhow!("发送 403 失败: {}", e))?;
                    return Ok(());
                }

                // 认证成功，清除 nonce
                self.nonce_store.remove(&device_id);
                debug!(device_id = %device_id, "🔐 摘要认证通过");
                next(tx).await
            }
        }
    }
}
