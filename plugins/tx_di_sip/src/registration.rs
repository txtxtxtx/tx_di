//! 注册状态模型与注册句柄
//!
//! - [`SipRegistration`]：单个账号的注册状态（L0 纯净，无 GB 语义）
//! - [`SipRegistrationStore`]：注册状态注册表（DI 组件，admin 后台可查询）
//! - [`RegistrationHandle`]：持有单例 [`Registration`] 的注册句柄，
//!   跨周期复用（NAT 公网地址学习、固定 Call-ID 不丢失）

use dashmap::DashMap;
use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip::HostWithPort;
use rsipstack::transaction::endpoint::EndpointInnerRef;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tx_di_core::{Component, DepsTuple, RIE};

use crate::SipErr;

/// 注册状态（可序列化，供 admin 展示）
#[derive(Clone, Debug, serde::Serialize)]
pub struct SipRegistration {
    /// 注册账号（通常为设备/平台 ID）
    pub username: String,
    /// 注册服务器
    pub registrar: String,
    /// 当前是否注册成功
    pub registered: bool,
    /// 注册有效期（秒）
    pub expires: u32,
    /// NAT 学习到的公网地址（`ip:port`）
    pub public_addr: Option<String>,
    /// 最近成功时间（Unix 秒）
    pub last_success: Option<i64>,
    /// 最近错误
    pub last_error: Option<String>,
    /// 连续失败次数
    pub fail_count: u32,
}

impl SipRegistration {
    fn new(username: &str, registrar: &str) -> Self {
        Self {
            username: username.to_string(),
            registrar: registrar.to_string(),
            registered: false,
            expires: 0,
            public_addr: None,
            last_success: None,
            last_error: None,
            fail_count: 0,
        }
    }
}

/// 注册状态注册表（DI 组件）
///
/// 各 SipClient / 上层插件写入，admin 后台注入查询：
/// ```rust,ignore
/// #[component(...)]
/// struct Admin { store: Arc<SipRegistrationStore> }
/// store.all() // → Vec<SipRegistration>
/// ```
#[derive(Component, Default)]
#[component(init_sort = 10000)]
pub struct SipRegistrationStore {
    #[tx_cst(DashMap::new())]
    regs: DashMap<String, SipRegistration>,
}

impl SipRegistrationStore {
    /// 记录注册成功
    pub fn mark_success(&self, username: &str, registrar: &str, expires: u32, public: Option<&str>) {
        let mut entry = self
            .regs
            .entry(username.to_string())
            .or_insert_with(|| SipRegistration::new(username, registrar));
        entry.registered = true;
        entry.expires = expires;
        entry.public_addr = public.map(|s| s.to_string());
        entry.last_success = Some(now_unix());
        entry.last_error = None;
        entry.fail_count = 0;
    }

    /// 记录注册失败
    pub fn mark_failed(&self, username: &str, registrar: &str, err: &str) {
        let mut entry = self
            .regs
            .entry(username.to_string())
            .or_insert_with(|| SipRegistration::new(username, registrar));
        entry.registered = false;
        entry.last_error = Some(err.to_string());
        entry.fail_count += 1;
    }

    /// 注销
    pub fn mark_unregistered(&self, username: &str) {
        if let Some(mut entry) = self.regs.get_mut(username) {
            entry.registered = false;
        }
    }

    /// 查询单账号
    pub fn get(&self, username: &str) -> Option<SipRegistration> {
        self.regs.get(username).map(|v| v.clone())
    }

    /// 全部注册状态
    pub fn all(&self) -> Vec<SipRegistration> {
        self.regs.iter().map(|kv| kv.value().clone()).collect()
    }

    /// 数量
    pub fn len(&self) -> usize {
        self.regs.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.regs.is_empty()
    }
}

/// 注册句柄：持有**单例** [`Registration`]，跨周期复用
///
/// 与 `SipSender::register`（每次新建 Registration）不同，本句柄保证：
/// - NAT 公网地址学习结果持续（`discovered_public()`）
/// - Call-ID 固定（真实设备重注册一致性）
/// - 401/407 自动重认证由 rsipstack 处理
#[derive(Clone)]
pub struct RegistrationHandle {
    reg: Arc<Mutex<Registration>>,
    /// 注册账号
    pub username: String,
    /// 注册服务器 URI（`sip:host:port`）
    pub registrar: String,
    /// 注册有效期（秒）
    pub expires: u32,
}

impl RegistrationHandle {
    /// 创建句柄（不发起注册；需在 Endpoint 就绪后调用）
    pub fn new(endpoint: EndpointInnerRef, registrar: &str, username: &str, password: &str, realm: Option<String>, expires: u32) -> Self {
        let registrar_uri = normalize_registrar(registrar);
        let credential = Credential {
            username: username.to_string(),
            password: password.to_string(),
            realm,
        };
        let reg = Registration::new(endpoint, Some(credential));
        Self {
            reg: Arc::new(Mutex::new(reg)),
            username: username.to_string(),
            registrar: registrar_uri,
            expires,
        }
    }

    /// 发起注册（或续期）
    pub async fn register(&self) -> RIE<rsipstack::sip::Response> {
        let uri = rsipstack::sip::Uri::try_from(self.registrar.as_str())
            .map_err(|_| SipErr::InvalidUri)?;
        let mut reg = self.reg.lock().await;
        reg.register(uri, Some(self.expires))
            .await
            .map_err(|_| SipErr::RegisterFailed.into())
    }

    /// 注销（Expires: 0）
    pub async fn unregister(&self) -> RIE<rsipstack::sip::Response> {
        let uri = rsipstack::sip::Uri::try_from(self.registrar.as_str())
            .map_err(|_| SipErr::InvalidUri)?;
        let mut reg = self.reg.lock().await;
        reg.register(uri, Some(0))
            .await
            .map_err(|_| SipErr::RegisterFailed.into())
    }

    /// NAT 学习到的公网地址（注册成功后可用）
    pub fn discovered_public(&self) -> Option<HostWithPort> {
        // Mutex 快速访问
        self.reg.try_lock().ok()?.discovered_public_address()
    }

    /// 服务端确认的有效期
    pub fn expires(&self) -> u32 {
        self.reg.try_lock().ok().map(|r| r.expires()).unwrap_or(self.expires)
    }
}

/// 规范化注册服务器地址（补 `sip:` 前缀）
fn normalize_registrar(registrar: &str) -> String {
    if registrar.starts_with("sip:") || registrar.starts_with("sips:") {
        registrar.to_string()
    } else {
        format!("sip:{}", registrar)
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// 占位：退避重连工具（供 SipClient 使用）
/// 计算第 `attempt` 次重试的等待时长（指数退避，上限 60s）
pub(crate) fn backoff_delay(attempt: u32, base_secs: u64) -> Duration {
    let base = base_secs.max(1);
    let exp = 1u64 << attempt.min(6); // 2^attempt，封顶 64
    Duration::from_secs((base * exp).min(60))
}

// ── 单元测试 ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_mark_success_and_query() {
        let store = SipRegistrationStore::default();
        store.mark_success("34020000001320000001", "sip:192.168.1.1:5060", 3600, Some("1.2.3.4:5060"));
        let r = store.get("34020000001320000001").expect("应查到");
        assert!(r.registered);
        assert_eq!(r.expires, 3600);
        assert_eq!(r.public_addr.as_deref(), Some("1.2.3.4:5060"));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn store_mark_failed_then_success_resets() {
        let store = SipRegistrationStore::default();
        store.mark_failed("u1", "sip:r", "401 Unauthorized");
        let r = store.get("u1").unwrap();
        assert!(!r.registered);
        assert_eq!(r.fail_count, 1);
        assert!(r.last_error.is_some());

        store.mark_success("u1", "sip:r", 3600, None);
        let r = store.get("u1").unwrap();
        assert!(r.registered);
        assert_eq!(r.fail_count, 0);
        assert!(r.last_error.is_none());
    }

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_delay(0, 2), Duration::from_secs(2));
        assert_eq!(backoff_delay(1, 2), Duration::from_secs(4));
        assert_eq!(backoff_delay(2, 2), Duration::from_secs(8));
        // 封顶 60s
        assert_eq!(backoff_delay(10, 2), Duration::from_secs(60));
    }

    #[test]
    fn normalize_registrar_variants() {
        assert_eq!(normalize_registrar("192.168.1.1:5060"), "sip:192.168.1.1:5060");
        assert_eq!(normalize_registrar("sip:192.168.1.1:5060"), "sip:192.168.1.1:5060");
        assert_eq!(normalize_registrar("sips:x.com"), "sips:x.com");
    }
}
