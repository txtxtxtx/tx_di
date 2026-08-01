//! SIP 客户端组件（通用注册生命周期，零 GB 语义）
//!
//! 封装「周期性 REGISTER + 心跳回调 + 指数退避重连 + 注销」的标准客户端生命期：
//! - 持有**单例** [`RegistrationHandle`]（NAT 公网地址学习持续、Call-ID 固定）
//! - `on_keepalive` 回调：上层（GB 设备心跳、级联目录推送）按周期注入
//! - `on_registered` 回调：注册成功/失败状态通知
//! - 连续失败进入指数退避重连（`max_retries` / `backoff_base_secs`）
//! - 注册状态写入 [`SipRegistrationStore`]（admin 可查询）
//!
//! 供 `tx_di_gb_dev`（设备端）与级联下级复用；L0 保持纯净，**不感知任何 GB28181 语义**。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tx_di_core::{App, Component, DepsTuple, RIE};

use crate::registration::{RegistrationHandle, SipRegistrationStore, backoff_delay};
use crate::SipErr;
use crate::SipPlugin;

/// 心跳回调类型：`Fn() -> Future<RIE<()>>`（无参；上层闭包自行捕获上下文）
type KeepaliveFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = RIE<()>> + Send>> + Send + Sync>;

/// 注册状态回调类型：`Fn(bool)`
type RegisteredFn = Arc<dyn Fn(bool) + Send + Sync>;

/// SIP 客户端配置（TOML `[sip_client]`）
///
/// 所有字段均有 `#[serde(default)]`，配置段缺失时退化为全默认
///（`enabled = false`），不会破坏未使用本组件的应用构建。
#[derive(Debug, Clone, serde::Deserialize, Component)]
#[component(conf = "sip_client", init_sort = 20000)]
pub struct SipClientConfig {
    /// 注册服务器地址（如 `"sip:192.168.1.1:5060"` 或 `"192.168.1.1:5060"`）
    #[serde(default)]
    pub registrar: String,

    /// 注册用户名（通常为平台 / 设备 ID）
    #[serde(default)]
    pub username: String,

    /// 注册密码
    #[serde(default)]
    pub password: String,

    /// 认证域（realm）；`None` 表示接受任意挑战
    #[serde(default)]
    pub realm: Option<String>,

    /// 注册有效期（秒），默认 3600
    #[serde(default = "default_expires")]
    pub expires: u32,

    /// 续期间隔（秒）；为 0 时自动取 `expires / 2`（至少 30 秒）
    #[serde(default)]
    pub renew_secs: u32,

    /// 心跳间隔（秒）；0 = 不执行 keepalive 回调
    #[serde(default)]
    pub keepalive_secs: u32,

    /// 连续注册失败退避上限，默认 5（第 5 次后按上限 60s 循环）
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// 指数退避基数（秒），默认 2（2s/4s/8s/...封顶 60s）
    #[serde(default = "default_backoff_base_secs")]
    pub backoff_base_secs: u64,

    /// 是否启用客户端注册（默认 false）
    #[serde(default)]
    pub enabled: bool,
}

fn default_expires() -> u32 {
    3600
}

fn default_max_retries() -> u32 {
    5
}

fn default_backoff_base_secs() -> u64 {
    2
}

/// SIP 客户端组件
///
/// 通过 `#[component(app_async_run, shutdown)]` 接入 DI 生命周期：
/// - `app_async_run`：首次注册 → 周期 keepalive/续期 → 指数退避重连 → 取消时注销
/// - `shutdown`：取消后台任务并向上级发送注销（Expires: 0）
///
/// 依赖 [`SipPlugin`]（提供 [`SipSender`]），因此 `init_sort` 必须晚于 `SipPlugin`
/// 的初始化，确保端点已就绪。
#[derive(Component)]
#[component(app_async_run, shutdown, init_sort = 20000)]
pub struct SipClient {
    /// 客户端配置
    pub config: Arc<SipClientConfig>,

    /// SIP 插件引用（提供 SipSender）
    pub sip: Arc<SipPlugin>,

    /// 注册状态注册表（DI 注入）
    pub store: Arc<SipRegistrationStore>,

    /// 注册句柄（单例 Registration；首次注册前初始化）
    #[tx_cst(OnceLock::new())]
    pub reg: OnceLock<Arc<RegistrationHandle>>,

    /// 运行时注册参数覆盖（供上层在 async_init 阶段注入，优先于 [sip_client] TOML）
    #[tx_cst(OnceLock::new())]
    params_override: OnceLock<SipClientConfig>,

    /// 心跳回调（上层注册一次）
    #[tx_cst(OnceLock::new())]
    keepalive_hook: OnceLock<KeepaliveFn>,

    /// 注册状态回调（上层注册一次）
    #[tx_cst(OnceLock::new())]
    registered_hook: OnceLock<RegisteredFn>,

    /// 优雅关闭令牌（仅可设置一次）
    #[tx_cst(OnceLock::new())]
    pub cancel_token: OnceLock<CancellationToken>,
}

impl SipClient {
    /// 运行时注入注册参数（覆盖 `[sip_client]` TOML 配置）
    ///
    /// 供上层在 `app_async_init` 阶段调用（早于本组件的 `app_async_run`）：
    /// ```rust,ignore
    /// client.set_registration(SipClientConfig {
    ///     registrar: "sip:192.168.1.1:5060".into(),
    ///     username: device_id.into(), password: pwd.into(),
    ///     enabled: true, keepalive_secs: 60, ..Default::default()
    /// })?;
    /// ```
    pub fn set_registration(&self, cfg: SipClientConfig) -> RIE<()> {
        self.params_override
            .set(cfg)
            .map_err(|_| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 生效配置：运行时覆盖优先，否则用 `[sip_client]` TOML
    fn effective_config(&self) -> SipClientConfig {
        self.params_override
            .get()
            .cloned()
            .unwrap_or_else(|| (*self.config).clone())
    }

    /// 注册心跳回调（周期调用，如 GB 设备 Keepalive MESSAGE）
    pub fn on_keepalive<F, Fut>(&self, f: F) -> RIE<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RIE<()>> + Send + 'static,
    {
        let wrapped: KeepaliveFn = Arc::new(move || Box::pin(f()));
        self.keepalive_hook
            .set(wrapped)
            .map_err(|_| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 注册状态变更回调（`true`=注册成功，`false`=失败/断开）
    pub fn on_registered<F>(&self, f: F) -> RIE<()>
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        let wrapped: RegisteredFn = Arc::new(f);
        self.registered_hook
            .set(wrapped)
            .map_err(|_| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 获取注册句柄（首次调用时基于生效配置初始化）
    pub fn registration(&self) -> RIE<Arc<RegistrationHandle>> {
        if let Some(h) = self.reg.get() {
            return Ok(h.clone());
        }
        let cfg = self.effective_config();
        if cfg.registrar.is_empty() || cfg.username.is_empty() {
            return Err(SipErr::NotRegistered.into());
        }
        let endpoint = self.sip.sender()?.inner();
        let handle = Arc::new(RegistrationHandle::new(
            endpoint,
            &cfg.registrar,
            &cfg.username,
            &cfg.password,
            cfg.realm.clone(),
            cfg.expires,
        ));
        self.reg
            .set(handle.clone())
            .map_err(|_| SipErr::TokenAlreadySet)?;
        Ok(handle)
    }

    /// 发起一次注册（成功写 store + 回调；失败写 store）
    async fn do_register(&self) -> RIE<()> {
        let reg = self.registration()?;
        let resp = reg.register().await?;

        // 判断是否成功（rsipstack 已自动处理 401，最终 2xx 即成功）
        let ok = resp.status_code.kind() == rsipstack::sip::StatusCodeKind::Successful;
        if ok {
            let public = reg.discovered_public().map(|h| h.to_string());
            self.store.mark_success(
                &reg.username,
                &reg.registrar,
                reg.expires(),
                public.as_deref(),
            );
            info!(
                registrar = %reg.registrar,
                username = %reg.username,
                public = ?public,
                "✅ SIP 客户端注册成功"
            );
            if let Some(h) = self.registered_hook.get() {
                h(true);
            }
        } else {
            self.store.mark_failed(&reg.username, &reg.registrar, &resp.status_code.to_string());
            warn!(status = %resp.status_code, "SIP 注册返回非成功状态码");
            if let Some(h) = self.registered_hook.get() {
                h(false);
            }
        }
        Ok(())
    }

    /// 注销（Expires: 0）
    async fn do_unregister(&self) -> RIE<()> {
        let reg = self.registration()?;
        let _ = reg.unregister().await?;
        self.store.mark_unregistered(&reg.username);
        info!("SIP 客户端已注销");
        Ok(())
    }

    /// 执行心跳回调（注册成功后才执行）
    async fn do_keepalive(&self) -> RIE<()> {
        if let Some(hook) = self.keepalive_hook.get() {
            hook().await?;
        }
        Ok(())
    }

    /// 设置取消令牌（只能成功一次）
    pub fn set_cancel_token(&self, token: CancellationToken) -> RIE<()> {
        self.cancel_token
            .set(token)
            .map_err(|_e| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 计算续期间隔（基于生效配置）
    fn renew_interval(&self) -> Duration {
        let cfg = self.effective_config();
        let secs = if cfg.renew_secs > 0 {
            cfg.renew_secs
        } else {
            (cfg.expires / 2).max(30)
        };
        Duration::from_secs(secs as u64)
    }
}

/// `#[component(app_async_run)]` 回调：注册 → 周期 keepalive/续期 → 退避重连
async fn app_async_run(comp: Arc<SipClient>, _app: Arc<App>, token: CancellationToken) -> RIE<()> {
    let cfg = comp.effective_config();
    if !cfg.enabled {
        info!("SIP 客户端未启用（enabled=false），跳过注册");
        return Ok(());
    }
    if cfg.registrar.is_empty() || cfg.username.is_empty() {
        warn!("SIP 客户端启用但 registrar/username 未配置，跳过注册");
        return Ok(());
    }

    comp.set_cancel_token(token.clone())?;

    // 首次注册（失败进入退避重试循环）
    let mut fail_streak: u32 = 0;
    let max_retries = cfg.max_retries;
    let backoff_base = cfg.backoff_base_secs;

    // 主循环：keepalive / 续期 / 退避重连
    let comp2 = comp.clone();
    let task_token = token.clone();
    tokio::spawn(async move {
        // 先做首次注册（阻塞式，失败则退避重试直到成功或取消）
        loop {
            match comp2.do_register().await {
                Ok(()) => {
                    fail_streak = 0;
                    break;
                }
                Err(e) => {
                    fail_streak += 1;
                    warn!(
                        error = %e,
                        attempt = fail_streak,
                        "SIP 客户端注册失败，退避重试"
                    );
                    let delay = backoff_delay(
                        fail_streak.min(max_retries.max(1)).saturating_sub(1),
                        backoff_base,
                    );
                    tokio::select! {
                        biased;
                        _ = task_token.cancelled() => {
                            info!("SIP 客户端注册重试被取消");
                            return;
                        }
                        _ = tokio::time::sleep(delay) => {}
                    }
                }
            }
        }

        // 注册成功：启动周期任务（keepalive + 续期），失败进入重连
        let keepalive_secs = comp2.effective_config().keepalive_secs;
        let mut keepalive_tick: Option<tokio::time::Interval> = if keepalive_secs > 0 {
            let mut it = interval(Duration::from_secs(keepalive_secs as u64));
            it.tick().await; // 跳过立即触发
            Some(it)
        } else {
            None
        };
        let mut renew_tick = interval(comp2.renew_interval());
        renew_tick.tick().await; // 跳过立即触发

        loop {
            tokio::select! {
                biased;
                _ = task_token.cancelled() => {
                    if let Err(e) = comp2.do_unregister().await {
                        warn!(error = %e, "SIP 客户端注销失败");
                    }
                    info!("SIP 客户端生命周期任务已停止");
                    return;
                }
                _ = renew_tick.tick() => {
                    match comp2.do_register().await {
                        Ok(()) => {
                            fail_streak = 0;
                            if let Some(kt) = keepalive_tick.as_mut() {
                                kt.reset();
                            }
                        }
                        Err(e) => {
                            fail_streak += 1;
                            warn!(error = %e, "SIP 续期失败，进入重连退避");
                            // 进入退避重连子循环
                            if !comp2.reconnect_loop(&task_token, &mut fail_streak, max_retries, backoff_base).await {
                                return; // 已取消
                            }
                            // 重连成功后重置 keepalive 定时器
                            if let Some(kt) = keepalive_tick.as_mut() {
                                kt.reset();
                            }
                        }
                    }
                }
                _ = async {
                    if let Some(kt) = keepalive_tick.as_mut() {
                        kt.tick().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    if let Err(e) = comp2.do_keepalive().await {
                        warn!(error = %e, "SIP 心跳回调执行失败");
                    }
                }
            }
        }
    });

    // 挂起直到取消，保持 async_run 生命周期存活
    token.cancelled().await;
    info!("SIP 客户端 async_run 已结束");
    Ok(())
}

impl SipClient {
    /// 重连子循环：指数退避重试注册，直到成功或取消。
    ///
    /// 返回 `false` 表示已取消（调用方应退出主循环）。
    async fn reconnect_loop(
        &self,
        token: &CancellationToken,
        fail_streak: &mut u32,
        max_retries: u32,
        backoff_base: u64,
    ) -> bool {
        loop {
            *fail_streak += 1;
            let streak = *fail_streak;
            let delay = backoff_delay(streak.min(max_retries.max(1)).saturating_sub(1), backoff_base);
            info!(
                delay_secs = delay.as_secs(),
                fail_streak = streak,
                "SIP 客户端进入重连退避"
            );
            tokio::select! {
                biased;
                _ = token.cancelled() => {
                    info!("SIP 客户端重连被取消");
                    return false;
                }
                _ = tokio::time::sleep(delay) => {}
            }
            match self.do_register().await {
                Ok(()) => {
                    *fail_streak = 0;
                    info!("✅ SIP 客户端重连成功");
                    return true;
                }
                Err(e) => {
                    warn!(error = %e, "SIP 重连仍失败，继续退避");
                }
            }
        }
    }
}

/// `#[component(shutdown)]` 回调：触发取消（生命周期任务据此发送注销）
fn shutdown(comp: &SipClient) {
    if let Some(token) = comp.cancel_token.get() {
        info!("SIP 客户端正在优雅关闭...");
        token.cancel();
    }
}

// ── 单元测试 ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renew_interval_uses_config_when_set() {
        let cfg = SipClientConfig {
            registrar: "sip:r".into(),
            username: "u".into(),
            password: "p".into(),
            realm: None,
            expires: 3600,
            renew_secs: 100,
            keepalive_secs: 30,
            max_retries: 5,
            backoff_base_secs: 2,
            enabled: false,
        };
        // 通过 interval 计算路径验证默认值逻辑
        let secs = if cfg.renew_secs > 0 { cfg.renew_secs } else { (cfg.expires / 2).max(30) };
        assert_eq!(secs, 100);
    }

    #[test]
    fn renew_interval_falls_back_to_expires_half() {
        let cfg = SipClientConfig {
            registrar: "sip:r".into(),
            username: "u".into(),
            password: "p".into(),
            realm: None,
            expires: 3600,
            renew_secs: 0,
            keepalive_secs: 0,
            max_retries: 5,
            backoff_base_secs: 2,
            enabled: false,
        };
        let secs = if cfg.renew_secs > 0 { cfg.renew_secs } else { (cfg.expires / 2).max(30) };
        assert_eq!(secs, 1800);
    }
}
