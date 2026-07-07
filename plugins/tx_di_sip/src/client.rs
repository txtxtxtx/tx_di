//! SIP 客户端组件（通用注册生命周期，零 GB 语义）
//!
//! 封装「周期性 REGISTER + 续期 + 注销」的标准客户端生命期，
//! 复用 [`SipSender::register`] / [`SipSender::unregister`]
//! （rsipstack `Credential` 自动处理 401 重认证）。
//!
//! 供 `tx_di_gb_dev`（设备端）与级联下级复用；L0 保持纯净，**不感知任何 GB28181 语义**。
//! GB 心跳 MESSAGE 等内容由上层（L2b / 级联）通过 [`SipSender::send_message`] 自行管理。

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tx_di_core::{App, Component, DepsTuple, RIE};

use crate::SipErr;
use crate::SipPlugin;

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

    /// 是否启用客户端注册（默认 false）
    #[serde(default)]
    pub enabled: bool,
}

fn default_expires() -> u32 {
    3600
}

/// SIP 客户端组件
///
/// 通过 `#[component(app_async_run, shutdown)]` 接入 DI 生命周期：
/// - `app_async_run`：完成首次注册并启动续期后台任务
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

    /// 优雅关闭令牌（仅可设置一次）
    #[tx_cst(OnceLock::new())]
    pub cancel_token: OnceLock<CancellationToken>,
}

impl SipClient {
    /// 向上级注册（或续期）
    async fn do_register(&self) -> RIE<()> {
        let sender = self.sip.sender()?;
        sender
            .register(&self.config.registrar, &self.config.username, &self.config.password)
            .await?;
        info!(
            registrar = %self.config.registrar,
            username = %self.config.username,
            "SIP 客户端注册/续期成功"
        );
        Ok(())
    }

    /// 向上级注销（Expires: 0）
    async fn do_unregister(&self) -> RIE<()> {
        let sender = self.sip.sender()?;
        sender
            .unregister(&self.config.registrar, &self.config.username, &self.config.password)
            .await?;
        info!("SIP 客户端已向上级注销");
        Ok(())
    }

    /// 设置取消令牌（只能成功一次）
    pub fn set_cancel_token(&self, token: CancellationToken) -> RIE<()> {
        self.cancel_token
            .set(token)
            .map_err(|_e| SipErr::TokenAlreadySet)?;
        Ok(())
    }

    /// 计算续期间隔
    fn renew_interval(&self) -> Duration {
        let secs = if self.config.renew_secs > 0 {
            self.config.renew_secs
        } else {
            (self.config.expires / 2).max(30)
        };
        Duration::from_secs(secs as u64)
    }
}

/// `#[component(app_async_run)]` 回调：完成首次注册并启动续期循环
async fn app_async_run(comp: Arc<SipClient>, _app: Arc<App>, token: CancellationToken) -> RIE<()> {
    if !comp.config.enabled {
        info!("SIP 客户端未启用（enabled=false），跳过注册");
        return Ok(());
    }

    comp.set_cancel_token(token.clone())?;

    // 首次注册（失败不致命，等待续期周期重试）
    if let Err(e) = comp.do_register().await {
        warn!(error = %e, "SIP 客户端初次注册失败，将在续期周期重试");
    }

    // 续期后台任务：周期性 REGISTER，取消时注销
    let comp2 = comp.clone();
    let task_token = token.clone();
    tokio::spawn(async move {
        let mut ticker = interval(comp2.renew_interval());
        loop {
            tokio::select! {
                biased;
                _ = task_token.cancelled() => {
                    if let Err(e) = comp2.do_unregister().await {
                        warn!(error = %e, "SIP 客户端注销失败");
                    }
                    info!("SIP 客户端续期任务已停止");
                    return;
                }
                _ = ticker.tick() => {
                    if let Err(e) = comp2.do_register().await {
                        warn!(error = %e, "SIP 客户端续期注册失败");
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

/// `#[component(shutdown)]` 回调：触发取消（续期任务据此发送注销）
fn shutdown(comp: &SipClient) {
    if let Some(token) = comp.cancel_token.get() {
        info!("SIP 客户端正在优雅关闭...");
        token.cancel();
    }
}
