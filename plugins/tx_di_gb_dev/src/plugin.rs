//! 设备端主组件
//!
//! [`Gb28181Device`] 是设备端核心组件：
//! - `app_async_init`：向 [`SipPlugin`] 注册 MESSAGE / INVITE / BYE 处理器；
//!   并把注册参数注入 [`SipClient`]（注册/心跳/重连/注销由 SipClient 统一管理）；
//! - `app_async_run`：仅与取消令牌同步（生命周期已交给 SipClient）；
//! - `shutdown`：触发取消令牌。
//!
//! 收到平台下发的查询/控制后，经 [`crate::handler::DeviceHandler`] 取业务数据并回网。

use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::OnceLock;

use tokio_util::sync::CancellationToken;
use tracing::info;
use tx_di_core::{App, Component, DepsTuple, RIE};
use tx_di_sip::{SipClient, SipClientConfig, SipPlugin, SipTx};

use crate::config::GbDevConfig;
use crate::handler::{DeviceHandler, NoopDeviceHandler};
use crate::register::registrar_of;

/// GB28181 设备端组件
///
/// 依赖 [`SipPlugin`]（提供 SIP 端点与发送器）与可选 [`DeviceHandler`]
/// （业务回调；未提供时使用 [`NoopDeviceHandler`] 兜底）。
///
/// **注册/心跳生命周期由 [`SipClient`] 管理**：本组件在 `app_async_init`
/// 阶段把注册参数注入 SipClient 并注册心跳回调，不再自持注册循环。
#[derive(Component)]
#[component(app_async_init, app_async_run, shutdown, init_sort = 30000)]
pub struct Gb28181Device {
    /// 设备端配置
    pub config: Arc<GbDevConfig>,

    /// SIP 插件引用（提供 SipSender 与 SipRouter）
    pub sip: Arc<SipPlugin>,

    /// 通用注册客户端（负责 REGISTER 周期/心跳/重连/注销）
    pub sip_client: Arc<SipClient>,

    /// 业务回调（可选项：无 provider 时退化为 no-op）
    pub handler: Option<Arc<dyn DeviceHandler>>,

    /// 出网报文序号（心跳/查询响应自增）
    #[tx_cst(Arc::new(AtomicU32::new(1)))]
    pub sn: Arc<AtomicU32>,

    /// 优雅关闭令牌（仅可设置一次）
    #[tx_cst(OnceLock::new())]
    cancel_token: OnceLock<CancellationToken>,
}

impl Gb28181Device {
    /// 获取业务回调（未注入时返回 no-op 兜底）
    pub(crate) fn handler(&self) -> Arc<dyn DeviceHandler> {
        self.handler
            .clone()
            .unwrap_or_else(|| Arc::new(NoopDeviceHandler))
    }

    /// 分配下一个 SN（单调递增）
    pub(crate) fn next_sn(&self) -> u32 {
        self.sn.fetch_add(1, Ordering::SeqCst)
    }

    /// 设置取消令牌（只能成功一次）
    pub fn set_cancel_token(&self, token: CancellationToken) -> RIE<()> {
        self.cancel_token
            .set(token)
            .map_err(|_e| tx_di_sip::SipErr::TokenAlreadySet)?;
        Ok(())
    }
}

/// `#[component(app_async_init)]` 回调：注册 SIP 消息处理器 + 配置 SipClient
async fn app_async_init(comp: Arc<Gb28181Device>, _app: Arc<App>) -> RIE<()> {
    let sip = comp.sip.clone();

    // MESSAGE — 平台下发的目录/设备信息/状态查询与 PTZ 控制
    let dev = comp.clone();
    sip.add_handler(Some("MESSAGE"), 0, move |tx: SipTx| {
        let dev = dev.clone();
        async move { crate::register::handle_device_message(&dev, &tx).await }
    })?;

    // INVITE — 点播 / 语音广播（UAS）
    let dev = comp.clone();
    sip.add_handler(Some("INVITE"), 0, move |tx: SipTx| {
        let dev = dev.clone();
        async move { crate::invite::handle_invite(&dev, &tx).await }
    })?;

    // BYE — 挂断
    let dev = comp.clone();
    sip.add_handler(Some("BYE"), 0, move |tx: SipTx| {
        let dev = dev.clone();
        async move { crate::invite::handle_bye(&dev, &tx).await }
    })?;

    info!("Gb28181Device 已注册 SIP 处理器（MESSAGE/INVITE/BYE）");

    // ── 注册/心跳生命周期交给 SipClient（运行时注入参数，优先于 [sip_client] TOML）
    if comp.config.enabled {
        let client = comp.sip_client.clone();
        let username = if comp.config.username.is_empty() {
            comp.config.device_id.clone()
        } else {
            comp.config.username.clone()
        };
        let reg_cfg = SipClientConfig {
            registrar: registrar_of(&comp.config),
            username,
            password: comp.config.password.clone(),
            realm: comp.config.realm.clone(),
            expires: comp.config.register_ttl,
            renew_secs: 0,
            keepalive_secs: comp.config.heartbeat_secs,
            max_retries: 5,
            backoff_base_secs: 2,
            enabled: true,
        };
        client.set_registration(reg_cfg)?;

        // 心跳回调：SipClient 按 keepalive_secs 周期调用 → 发 GB Keepalive MESSAGE
        let dev = comp.clone();
        client.on_keepalive(move || {
            let dev = dev.clone();
            async move { dev.do_keepalive().await }
        })?;
        info!("Gb28181Device 注册/心跳生命周期已托管给 SipClient");
    }

    Ok(())
}

/// `#[component(app_async_run)]` 回调：仅与取消令牌同步（生命周期由 SipClient 管理）
async fn app_async_run(
    comp: Arc<Gb28181Device>,
    _app: Arc<App>,
    token: CancellationToken,
) -> RIE<()> {
    if !comp.config.enabled {
        info!("Gb28181Device 未启用（enabled=false），跳过");
        return Ok(());
    }
    comp.set_cancel_token(token.clone())?;
    // 注册/心跳/注销由 SipClient 的后台任务执行；这里挂起保持组件存活
    token.cancelled().await;
    info!("Gb28181Device async_run 已结束");
    Ok(())
}

/// `#[component(shutdown)]` 回调：触发取消
fn shutdown(comp: &Gb28181Device) {
    if let Some(token) = comp.cancel_token.get() {
        info!("Gb28181Device 正在优雅关闭...");
        token.cancel();
    }
}
