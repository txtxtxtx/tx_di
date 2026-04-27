//! GB28181 服务端插件主体
//!
//! `Gb28181Server` 是整个服务端的门面，集成到 tx-di 框架中：
//! - 持有全局 `DeviceRegistry`
//! - 提供主动查询（目录查询、设备信息查询）
//! - 提供主动点播（INVITE 到设备）
//! - 提供事件订阅接口
//! - 后台运行心跳超时检测

use crate::config::Gb28181ServerConfig;
use crate::device_registry::{DeviceInfo, DeviceRegistry};
use crate::event::{self, Gb28181Event};
use crate::handlers::register_server_handlers;
use crate::sdp::build_invite_sdp;
use crate::xml::build_catalog_query_xml;
use rsipstack::dialog::dialog::DialogState;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::invitation::InviteOption;
use rsipstack::sip as rsip;
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use tx_di_core::{tx_comp, App, BoxFuture, BuildContext, CompInit, RIE};
use tx_di_sip::SipPlugin;

/// 全局服务实例
static INSTANCE: OnceLock<Arc<Gb28181ServerInner>> = OnceLock::new();

struct Gb28181ServerInner {
    config: Arc<Gb28181ServerConfig>,
    registry: DeviceRegistry,
    sn: AtomicU32,
}

/// GB28181 服务端插件
///
/// # 配置文件
/// ```toml
/// [gb28181_server_config]
/// platform_id            = "34020000002000000001"
/// realm                  = "3402000000"
/// heartbeat_timeout_secs = 120
/// enable_auth            = false
///
/// [gb28181_server_config.media]
/// local_ip       = "192.168.1.200"
/// rtp_port_start = 10000
/// rtp_port_end   = 20000
/// ```
///
/// # 使用示例
///
/// ```rust,no_run
/// use tx_di_gb28181::{Gb28181Server, Gb28181Event};
/// use tx_di_core::BuildContext;
///
/// // 1. 订阅事件
/// Gb28181Server::on_event(|ev| async move {
///     if let Gb28181Event::DeviceRegistered { device_id, .. } = ev {
///         println!("设备上线: {device_id}");
///     }
///     Ok(())
/// });
///
/// // 2. 启动（自动 init SIP + 注册 handlers + 启动心跳检测）
/// let mut ctx = BuildContext::new(Some("gb28181-server.toml"));
/// ctx.build().await.unwrap();
///
/// // 3. 主动操作
/// let srv = Gb28181Server::instance();
/// srv.query_catalog("34020000001320000001").await.unwrap();
/// srv.invite("34020000001320000001", "192.168.1.200", 10000).await.unwrap();
/// ```
#[tx_comp(init)]
pub struct Gb28181Server {
    pub config: Arc<Gb28181ServerConfig>,
}

impl CompInit for Gb28181Server {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        info!(
            platform_id = %self.config.platform_id,
            realm = %self.config.realm,
            "GB28181 服务端插件初始化中..."
        );
        Ok(())
    }

    fn async_init(ctx: Arc<App>) -> BoxFuture<'static, RIE<()>> {
        Box::pin(async move {
            let config = ctx.inject::<Gb28181ServerConfig>();

            // 创建全局注册表
            let registry = DeviceRegistry::new();

            // 注册 SIP 消息处理器
            register_server_handlers(Arc::new(registry.clone()));

            info!(
                platform_id = %config.platform_id,
                heartbeat_timeout_secs = config.heartbeat_timeout_secs,
                "✅ GB28181 服务端处理器注册完成"
            );

            // 存储全局实例
            let inner = Arc::new(Gb28181ServerInner {
                config: config.clone(),
                registry,
                sn: AtomicU32::new(1),
            });
            let _ = INSTANCE.set(inner.clone());

            // 启动心跳超时检测后台任务
            let timeout_secs = config.heartbeat_timeout_secs;
            tokio::spawn(async move {
                heartbeat_watchdog(inner, timeout_secs).await;
            });

            Ok(())
        })
    }

    fn init_sort() -> i32 {
        // 在 SipPlugin（MAX-1）之后初始化
        i32::MAX
    }
}

impl Gb28181Server {
    // ── 门面方法 ────────────────────────────────────────────────────────────

    /// 获取全局实例（需在 build().await 完成后调用）
    pub fn instance() -> Gb28181ServerHandle {
        let inner = INSTANCE
            .get()
            .expect("Gb28181Server 尚未初始化，请确保 ctx.build().await 已完成")
            .clone();
        Gb28181ServerHandle { inner }
    }

    /// 订阅 GB28181 事件
    ///
    /// 必须在 `ctx.build().await` 之前调用。
    pub fn on_event<F, Fut>(handler: F)
    where
        F: Fn(Gb28181Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        event::add_event_listener(handler);
    }
}

// ── 心跳超时检测后台任务 ──────────────────────────────────────────────────────

async fn heartbeat_watchdog(inner: Arc<Gb28181ServerInner>, timeout_secs: u64) {
    // 每 30 秒检查一次
    let check_interval = Duration::from_secs(timeout_secs.min(30));
    let mut ticker = interval(check_interval);
    ticker.tick().await; // 跳过立即触发的第一次

    loop {
        ticker.tick().await;
        let timeout_devices = inner.registry.check_timeouts(timeout_secs);
        for device_id in timeout_devices {
            warn!(device_id = %device_id, "⚠️ 设备心跳超时，标记离线");
            inner.registry.set_offline(&device_id);
            tokio::spawn(event::emit(Gb28181Event::DeviceOffline {
                device_id: device_id.clone(),
            }));
        }
    }
}

// ── 操作句柄 ──────────────────────────────────────────────────────────────────

/// GB28181 服务端操作句柄（通过 `Gb28181Server::instance()` 获取）
#[derive(Clone)]
pub struct Gb28181ServerHandle {
    inner: Arc<Gb28181ServerInner>,
}

impl Gb28181ServerHandle {
    // ── 注册表查询 ───────────────────────────────────────────────────────────

    /// 获取设备信息
    pub fn get_device(&self, device_id: &str) -> Option<DeviceInfo> {
        self.inner.registry.get(device_id)
    }

    /// 获取所有在线设备
    pub fn online_devices(&self) -> Vec<DeviceInfo> {
        self.inner.registry.online_devices()
    }

    /// 获取注册设备总数
    pub fn device_count(&self) -> usize {
        self.inner.registry.total_count()
    }

    /// 获取在线设备数
    pub fn online_count(&self) -> usize {
        self.inner.registry.online_count()
    }

    // ── 主动查询 ─────────────────────────────────────────────────────────────

    /// 向设备发送目录查询（MESSAGE Catalog）
    ///
    /// 设备收到后会回复包含通道列表的 MESSAGE，触发 `Gb28181Event::CatalogReceived`。
    pub async fn query_catalog(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self
            .inner
            .registry
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("设备 {} 未注册", device_id))?;

        let sn = self.inner.sn.fetch_add(1, Ordering::Relaxed);
        let xml = build_catalog_query_xml(&self.inner.config.platform_id, sn);

        self.send_message_to_device(&dev.contact, &xml, "Application/MANSCDP+xml", sn)
            .await?;

        info!(device_id = %device_id, sn = sn, "📂 发送目录查询");
        Ok(())
    }

    /// 向设备发送设备信息查询（MESSAGE DeviceInfo）
    pub async fn query_device_info(&self, device_id: &str) -> anyhow::Result<()> {
        let dev = self
            .inner
            .registry
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("设备 {} 未注册", device_id))?;

        let sn = self.inner.sn.fetch_add(1, Ordering::Relaxed);
        let xml =
            crate::xml::build_device_info_query_xml(&self.inner.config.platform_id, device_id, sn);

        self.send_message_to_device(&dev.contact, &xml, "Application/MANSCDP+xml", sn)
            .await?;

        info!(device_id = %device_id, "ℹ️ 发送设备信息查询");
        Ok(())
    }

    // ── 点播控制 ─────────────────────────────────────────────────────────────

    /// 向设备发起实时点播（INVITE）
    ///
    /// # 参数
    /// - `device_id`：目标设备 ID（也是通道 ID）
    /// - `rtp_recv_ip`：媒体服务器接收 RTP 的 IP
    /// - `rtp_recv_port`：媒体服务器接收 RTP 的端口
    ///
    /// # 返回
    /// 返回 `call_id`，用于后续 BYE 操作。
    ///
    /// # 事件
    /// - 会话建立后触发 `Gb28181Event::SessionStarted`
    /// - 会话结束后触发 `Gb28181Event::SessionEnded`
    pub async fn invite(
        &self,
        device_id: &str,
        rtp_recv_ip: &str,
        rtp_recv_port: u16,
    ) -> anyhow::Result<String> {
        self.invite_internal(device_id, rtp_recv_ip, rtp_recv_port, true)
            .await
    }

    /// 向设备发起历史回放（INVITE s=Playback）
    pub async fn invite_playback(
        &self,
        device_id: &str,
        rtp_recv_ip: &str,
        rtp_recv_port: u16,
    ) -> anyhow::Result<String> {
        self.invite_internal(device_id, rtp_recv_ip, rtp_recv_port, false)
            .await
    }

    async fn invite_internal(
        &self,
        device_id: &str,
        rtp_recv_ip: &str,
        rtp_recv_port: u16,
        is_realtime: bool,
    ) -> anyhow::Result<String> {
        let dev = self
            .inner
            .registry
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("设备 {} 未注册或已离线", device_id))?;

        let sender = SipPlugin::sender();
        let endpoint = sender.inner();

        // 构建 SDP offer（媒体服务器接收 RTP）
        let ssrc = format!("{:010}", self.inner.sn.fetch_add(1, Ordering::Relaxed));
        let sdp_offer =
            build_invite_sdp(rtp_recv_ip, rtp_recv_port, &ssrc, is_realtime);

        let platform_id = &self.inner.config.platform_id;
        let sip_config = endpoint
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {}", e))?;

        // caller = 平台 URI，callee = 设备联系地址
        let caller_str = format!("sip:{}@{}", platform_id, "192.168.1.200"); // 实际应从 SipConfig 读
        let callee_str = dev.contact.clone();

        info!(
            device_id = %device_id,
            callee = %callee_str,
            rtp = %format!("{}:{}", rtp_recv_ip, rtp_recv_port),
            "📹 发起点播 INVITE"
        );

        // 使用 DialogLayer 发起 INVITE
        let dialog_layer = Arc::new(DialogLayer::new(endpoint.clone()));
        let (state_tx, mut state_rx) = dialog_layer.new_dialog_state_channel();

        let caller_uri = rsip::Uri::try_from(caller_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的主叫 URI: {}", e))?;
        let callee_uri = rsip::Uri::try_from(callee_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的被叫 URI: {}", e))?;

        let invite_option = InviteOption {
            caller: caller_uri.clone(),
            callee: callee_uri,
            contact: caller_uri,
            content_type: Some("application/sdp".to_string()),
            offer: Some(sdp_offer.into_bytes().into()),
            credential: None,
            ..Default::default()
        };

        let (dialog, _resp) = dialog_layer
            .do_invite(invite_option, state_tx)
            .await
            .map_err(|e| anyhow::anyhow!("INVITE 失败: {}", e))?;

        let call_id = dialog.id().call_id.clone();
        let call_id_clone = call_id.clone();
        let device_id_owned = device_id.to_string();
        let device_id_ended = device_id.to_string();
        let channel_id = device_id.to_string();

        // 在独立任务监听对话状态
        tokio::spawn(async move {
            while let Some(state) = state_rx.recv().await {
                match state {
                    DialogState::Confirmed(id, _resp) => {
                        info!(call_id = %call_id_clone, dialog_id = %id, "✅ 点播会话已确认");
                        tokio::spawn(event::emit(Gb28181Event::SessionStarted {
                            device_id: device_id_owned.clone(),
                            channel_id: channel_id.clone(),
                            call_id: call_id_clone.clone(),
                        }));
                    }
                    DialogState::Terminated(id, reason) => {
                        info!(
                            call_id = %call_id_clone,
                            dialog_id = %id,
                            reason = ?reason,
                            "📹 点播会话结束"
                        );
                        tokio::spawn(event::emit(Gb28181Event::SessionEnded {
                            device_id: device_id_ended.clone(),
                            channel_id: device_id_ended.clone(),
                            call_id: call_id_clone.clone(),
                        }));
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(call_id)
    }

    // ── 内部工具 ─────────────────────────────────────────────────────────────

    /// 向指定设备 Contact URI 发送 MESSAGE
    async fn send_message_to_device(
        &self,
        contact: &str,
        body: &str,
        content_type: &str,
        seq: u32,
    ) -> anyhow::Result<()> {
        let sender = SipPlugin::sender();
        let inner = sender.inner();

        let req_uri = rsip::Uri::try_from(contact)
            .map_err(|e| anyhow::anyhow!("无效的设备 Contact URI '{}': {}", contact, e))?;

        let platform_id = &self.inner.config.platform_id;
        // 平台的 From URI（简化：用 platform_id@本机 IP）
        let from_str = format!("sip:{}", platform_id);
        let from_uri = rsip::Uri::try_from(from_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的 From URI: {}", e))?;

        let via = inner
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {}", e))?;

        let from = rsip::typed::From {
            display_name: None,
            uri: from_uri.into(),
            params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
                rsipstack::transaction::make_tag(),
            ))],
        };
        let to = rsip::typed::To {
            display_name: None,
            uri: req_uri.clone().into(),
            params: vec![],
        };

        let mut request = inner.make_request(
            rsip::method::Method::Message,
            req_uri,
            via,
            from,
            to,
            seq,
            None,
        );

        request
            .headers
            .push(rsip::Header::ContentType(content_type.into()));
        request.body = body.as_bytes().to_vec().into();

        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("构造事务 key 失败: {}", e))?;

        let mut tx = Transaction::new_client(key, request, inner, None);
        tx.send()
            .await
            .map_err(|e| anyhow::anyhow!("发送 MESSAGE 失败: {}", e))?;

        Ok(())
    }
}
