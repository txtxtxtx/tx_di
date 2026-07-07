//! 设备端主组件
//!
//! [`Gb28181Device`] 是设备端核心组件：
//! - `app_async_init`：向 [`SipPlugin`] 注册 MESSAGE / INVITE / BYE 处理器；
//! - `app_async_run`：运行注册 + 心跳生命周期（取消时注销）；
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
use tx_di_sip::{SipPlugin, SipTx};

use crate::config::GbDevConfig;
use crate::handler::{DeviceHandler, NoopDeviceHandler};

/// GB28181 设备端组件
///
/// 依赖 [`SipPlugin`]（提供 SIP 端点与发送器）与可选 [`DeviceHandler`]
/// （业务回调；未提供时使用 [`NoopDeviceHandler`] 兜底）。
#[derive(Component)]
#[component(app_async_init, app_async_run, shutdown, init_sort = 30000)]
pub struct Gb28181Device {
    /// 设备端配置
    pub config: Arc<GbDevConfig>,

    /// SIP 插件引用（提供 SipSender 与 SipRouter）
    pub sip: Arc<SipPlugin>,

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

/// `#[component(app_async_init)]` 回调：注册 SIP 消息处理器（快速、不阻塞）
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
    Ok(())
}

/// `#[component(app_async_run)]` 回调：运行注册 + 心跳生命周期
async fn app_async_run(
    comp: Arc<Gb28181Device>,
    _app: Arc<App>,
    token: CancellationToken,
) -> RIE<()> {
    if !comp.config.enabled {
        info!("Gb28181Device 未启用（enabled=false），跳过注册/心跳");
        return Ok(());
    }
    comp.set_cancel_token(token.clone())?;
    crate::register::run_lifecycle(&comp, token).await
}

/// `#[component(shutdown)]` 回调：触发取消（续期/心跳任务据此发送注销）
fn shutdown(comp: &Gb28181Device) {
    if let Some(token) = comp.cancel_token.get() {
        info!("Gb28181Device 正在优雅关闭...");
        token.cancel();
    }
}
