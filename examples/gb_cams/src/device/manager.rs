//! 多设备生命周期管理器
//!
//! 管理所有虚拟设备的：
//! - CRUD（创建/删除/查询）
//! - SIP 注册/注销/心跳
//! - SIP 消息处理（INVITE/MESSAGE/OPTIONS）

use super::virtual_device::{ChannelStatus, DeviceStatus, VirtualChannel, VirtualDevice};
use crate::config::GbCamsConfig;
use dashmap::DashMap;
use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::dialog_layer::DialogLayer;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip::{self as rsip, HeadersExt, StatusCode};
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use rsipstack::transport::udp::UdpConnection;
use rsipstack::transport::TransportLayer;
use rsipstack::EndpointBuilder;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// 设备事件（供 API 层 SSE 推送）
#[derive(Debug, Clone, serde::Serialize)]
pub enum DeviceEvent {
    Registered { device_id: String },
    RegisterFailed { device_id: String, reason: String },
    Unregistered { device_id: String },
    Keepalive { device_id: String },
    Offline { device_id: String },
    CatalogQuery { device_id: String, sn: u32 },
    InviteReceived { device_id: String, channel_id: String, call_id: String },
    InviteEnded { device_id: String, call_id: String },
}

/// 全局设备管理器单例
static INSTANCE: OnceLock<Arc<DeviceManager>> = OnceLock::new();

/// 设备管理器
pub struct DeviceManager {
    /// 所有虚拟设备
    pub devices: DashMap<String, VirtualDevice>,
    /// 全局配置
    pub config: Arc<GbCamsConfig>,
    /// 下一个可用的 SIP 端口
    next_port: AtomicU32,
    /// 事件广播
    event_tx: tokio::sync::broadcast::Sender<DeviceEvent>,
    /// 停止标志
    cancel: CancellationToken,
    /// 是否已启动
    running: AtomicBool,
}

impl DeviceManager {
    /// 初始化全局单例（需在 ctx.build() 之前调用）
    pub fn init(config: Arc<GbCamsConfig>, token: CancellationToken) {
        let (event_tx, _) = tokio::sync::broadcast::channel(1024);
        let next_port = config.sip_base_port;
        let mgr = Arc::new(Self {
            devices: DashMap::new(),
            config,
            next_port: AtomicU32::new(next_port as u32),
            event_tx,
            cancel: token,
            running: AtomicBool::new(false),
        });
        let _ = INSTANCE.set(mgr);
    }

    /// 获取全局实例
    pub fn instance() -> Arc<DeviceManager> {
        INSTANCE
            .get()
            .expect("DeviceManager 尚未初始化，请确保 init() 已调用")
            .clone()
    }

    /// 订阅设备事件
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<DeviceEvent> {
        self.event_tx.subscribe()
    }

    /// 发送事件
    pub fn emit(&self, event: DeviceEvent) {
        let _ = self.event_tx.send(event);
    }

    /// 分配下一个 SIP 端口
    fn alloc_port(&self) -> u16 {
        self.next_port.fetch_add(1, Ordering::Relaxed) as u16
    }

    // ── 设备 CRUD ──────────────────────────────────────────────────────────

    /// 添加设备（自动分配端口）
    pub fn add_device(
        &self,
        device_id: String,
        channels: Vec<(String, String)>,
        name: String,
    ) -> String {
        let port = self.alloc_port();
        let vchannels: Vec<VirtualChannel> = channels
            .into_iter()
            .map(|(id, name)| VirtualChannel {
                channel_id: id,
                name,
                status: ChannelStatus::Online,
            })
            .collect();

        let did = device_id.clone();
        let dev = VirtualDevice::new(device_id, vchannels, name, port);
        self.devices.insert(did.clone(), dev);
        did
    }

    /// 删除设备（自动注销）
    pub async fn remove_device(&self, device_id: &str) -> bool {
        if let Some((_, dev)) = self.devices.remove(device_id) {
            if dev.status == DeviceStatus::Registered {
                let _ = self.unregister_device(&dev).await;
            }
            self.emit(DeviceEvent::Unregistered {
                device_id: device_id.to_string(),
            });
            true
        } else {
            false
        }
    }

    /// 获取设备信息（克隆）
    pub fn get_device(&self, device_id: &str) -> Option<VirtualDevice> {
        self.devices.get(device_id).map(|r| r.clone())
    }

    /// 获取所有设备
    pub fn all_devices(&self) -> Vec<VirtualDevice> {
        self.devices.iter().map(|r| r.clone()).collect()
    }

    /// 在线设备数
    pub fn online_count(&self) -> usize {
        self.devices
            .iter()
            .filter(|r| r.status == DeviceStatus::Registered)
            .count()
    }

    /// 设备总数
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// 统计信息
    pub fn stats(&self) -> (usize, usize, usize) {
        let total = self.devices.len();
        let online = self.online_count();
        let channels: usize = self.devices.iter().map(|r| r.channels.len()).sum();
        (total, online, channels)
    }

    // ── 批量注册 + 心跳 ───────────────────────────────────────────────────

    /// 启动所有设备的注册 + 心跳循环
    pub fn start_all(&self) {
        if self.running.swap(true, Ordering::Relaxed) {
            // 已在运行，仅注册新增设备
            info!("设备管理器已在运行，跳过重复启动");
            return;
        }

        let mgr = INSTANCE.get().unwrap().clone();
        tokio::spawn(async move {
            for entry in mgr.devices.iter() {
                let device_id = entry.key().clone();
                let dev = entry.value().clone();
                let mgr_clone = mgr.clone();

                tokio::spawn(async move {
                    run_device_loop(mgr_clone, device_id, dev).await;
                });
            }

            info!(
                count = mgr.devices.len(),
                "🚀 所有设备已启动注册+心跳循环"
            );
        });
    }

    /// 注销设备
    async fn unregister_device(&self, dev: &VirtualDevice) -> anyhow::Result<()> {
        let local_addr: SocketAddr = format!("127.0.0.1:{}", dev.sip_port).parse()?;
        let transport = TransportLayer::new(self.cancel.child_token());
        let udp = UdpConnection::create_connection(local_addr, None, Some(self.cancel.child_token()))
            .await
            .map_err(|e| anyhow::anyhow!("绑定 UDP {} 失败: {}", local_addr, e))?;
        transport.add_transport(udp.into());

        let endpoint = EndpointBuilder::new()
            .with_cancel_token(self.cancel.child_token())
            .with_transport_layer(transport)
            .build();

        let credential = Credential {
            username: dev.username.clone(),
            password: self.config.password.clone(),
            realm: Some(self.config.realm.clone()),
        };

        let server_uri = rsip::Uri::try_from(self.config.platform_uri().as_str())
            .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

        let mut reg = Registration::new(endpoint.inner.clone(), Some(credential));
        let resp = reg.register(server_uri, Some(0)).await
            .map_err(|e| anyhow::anyhow!("注销失败: {}", e))?;

        info!(device_id = %dev.device_id, status = %resp.status_code, "注销响应");
        Ok(())
    }

    /// 停止所有设备
    pub fn stop_all(&self) {
        self.cancel.cancel();
        info!("设备管理器已停止");
    }
}

// ── 单设备注册+心跳循环 ──────────────────────────────────────────────────────

async fn run_device_loop(mgr: Arc<DeviceManager>, device_id: String, mut dev: VirtualDevice) {
    let local_addr: SocketAddr = match format!("0.0.0.0:{}", dev.sip_port).parse() {
        Ok(a) => a,
        Err(e) => {
            error!(device_id = %device_id, error = %e, "无效的本地地址");
            set_device_status(&mgr, &device_id, DeviceStatus::Failed, Some(e.to_string()));
            return;
        }
    };

    let transport = TransportLayer::new(mgr.cancel.child_token());

    let udp = match UdpConnection::create_connection(
        local_addr, None, Some(mgr.cancel.child_token()),
    ).await {
        Ok(u) => u,
        Err(e) => {
            error!(device_id = %device_id, error = %e, "绑定 UDP 失败");
            set_device_status(&mgr, &device_id, DeviceStatus::Failed, Some(e.to_string()));
            return;
        }
    };

    transport.add_transport(udp.into());

    let endpoint = EndpointBuilder::new()
        .with_cancel_token(mgr.cancel.child_token())
        .with_transport_layer(transport)
        .with_user_agent("GB-CAMS/1.0")
        .build();

    let endpoint_inner = endpoint.inner.clone();

    let mut incoming = match endpoint.incoming_transactions() {
        Ok(ch) => ch,
        Err(e) => {
            error!(device_id = %device_id, error = %e, "获取消息通道失败");
            set_device_status(&mgr, &device_id, DeviceStatus::Failed, Some(e.to_string()));
            return;
        }
    };

    let ep_inner = endpoint_inner.clone();
    let cancel_token = mgr.cancel.child_token();
    tokio::spawn(async move {
        tokio::select! {
            _ = ep_inner.serve() => {}
            _ = cancel_token.cancelled() => {}
        }
    });

    // ── 注册 ──────────────────────────────────────────────────────────────
    set_device_status(&mgr, &device_id, DeviceStatus::Registering, None);

    match do_register(&endpoint_inner, &mgr.config, &dev).await {
        Ok(()) => {
            set_device_status(&mgr, &device_id, DeviceStatus::Registered, None);
            dev.status = DeviceStatus::Registered;
            mgr.emit(DeviceEvent::Registered { device_id: device_id.clone() });
            info!(device_id = %device_id, sip_port = dev.sip_port, "✅ 设备注册成功");
        }
        Err(e) => {
            set_device_status(&mgr, &device_id, DeviceStatus::Failed, Some(e.to_string()));
            error!(device_id = %device_id, error = %e, "❌ 设备注册失败");
            mgr.emit(DeviceEvent::RegisterFailed {
                device_id: device_id.clone(),
                reason: e.to_string(),
            });
            return;
        }
    }

    // ── 心跳 + 消息处理 主循环 ────────────────────────────────────────────
    let heartbeat_dur = Duration::from_secs(mgr.config.heartbeat_secs);
    let refresh_dur = Duration::from_secs((mgr.config.register_ttl / 2).max(60) as u64);

    let mut heartbeat_ticker = interval(heartbeat_dur);
    let mut refresh_ticker = interval(refresh_dur);
    heartbeat_ticker.tick().await;
    refresh_ticker.tick().await;

    let device_id_clone = device_id.clone();

    loop {
        tokio::select! {
            _ = mgr.cancel.cancelled() => {
                info!(device_id = %device_id_clone, "收到停止信号");
                break;
            }

            _ = heartbeat_ticker.tick() => {
                if let Err(e) = send_keepalive(&endpoint_inner, &mgr.config, &dev).await {
                    warn!(device_id = %device_id_clone, error = %e, "心跳失败");
                } else {
                    dev.keepalive_count += 1;
                    dev.last_keepalive = Some(std::time::Instant::now());
                    if let Some(mut entry) = mgr.devices.get_mut(&device_id_clone) {
                        entry.keepalive_count = dev.keepalive_count;
                        entry.last_keepalive = dev.last_keepalive;
                    }
                    mgr.emit(DeviceEvent::Keepalive { device_id: device_id_clone.clone() });
                }
            }

            _ = refresh_ticker.tick() => {
                info!(device_id = %device_id_clone, "🔄 刷新注册");
                if let Err(e) = do_register(&endpoint_inner, &mgr.config, &dev).await {
                    error!(device_id = %device_id_clone, error = %e, "刷新注册失败");
                }
            }

            Some(tx) = incoming.recv() => {
                let did = device_id_clone.clone();
                let ep = endpoint_inner.clone();
                let cfg = mgr.config.clone();
                let dev_clone = dev.clone();
                let mgr_c = mgr.clone();
                tokio::spawn(async move {
                    handle_sip_message(mgr_c, did, ep, cfg, dev_clone, tx).await;
                });
            }
        }
    }

    let _ = mgr.unregister_device(&dev).await;
    mgr.emit(DeviceEvent::Unregistered { device_id: device_id_clone.clone() });
    info!(device_id = %device_id_clone, "设备已注销");
}

// ── SIP 注册 ─────────────────────────────────────────────────────────────────

async fn do_register(
    endpoint: &rsipstack::transaction::endpoint::EndpointInnerRef,
    config: &GbCamsConfig,
    dev: &VirtualDevice,
) -> anyhow::Result<()> {
    let credential = Credential {
        username: dev.username.clone(),
        password: config.password.clone(),
        realm: Some(config.realm.clone()),
    };

    let server_uri = rsip::Uri::try_from(config.platform_uri().as_str())
        .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

    let mut reg = Registration::new(endpoint.clone(), Some(credential));
    let resp = reg.register(server_uri, Some(config.register_ttl)).await
        .map_err(|e| anyhow::anyhow!("REGISTER 失败: {}", e))?;

    if resp.status_code == StatusCode::OK {
        Ok(())
    } else {
        Err(anyhow::anyhow!("注册失败，响应: {}", resp.status_code))
    }
}

// ── SIP 心跳 ─────────────────────────────────────────────────────────────────

async fn send_keepalive(
    endpoint: &rsipstack::transaction::endpoint::EndpointInnerRef,
    config: &GbCamsConfig,
    dev: &VirtualDevice,
) -> anyhow::Result<()> {
    let sn = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let xml = tx_di_gb28181::xml::build_keepalive_xml(&dev.device_id, sn);
    send_manscdp(endpoint, config, dev, &xml, sn).await
}

// ── SIP 消息处理 ─────────────────────────────────────────────────────────────

async fn handle_sip_message(
    mgr: Arc<DeviceManager>,
    device_id: String,
    endpoint: rsipstack::transaction::endpoint::EndpointInnerRef,
    config: Arc<GbCamsConfig>,
    dev: VirtualDevice,
    mut tx: Transaction,
) {
    let method_str = tx.original.method.to_string();

    match method_str.as_str() {
        "INVITE" => {
            handle_invite(mgr, device_id, tx).await;
        }
        "BYE" => {
            handle_bye(mgr, device_id, tx).await;
        }
        "MESSAGE" => {
            handle_message(mgr, device_id, endpoint, config, dev, tx).await;
        }
        "OPTIONS" => {
            let _ = tx.reply(StatusCode::OK).await;
        }
        _ => {
            warn!(method = %method_str, "未处理的 SIP 方法");
            let _ = tx.reply(StatusCode::MethodNotAllowed).await;
        }
    }
}

async fn handle_invite(mgr: Arc<DeviceManager>, device_id: String, tx: Transaction) {
    let call_id = tx.original.call_id_header()
        .map(|h| h.value().to_string())
        .unwrap_or_else(|_| "N/A".into());

    let sdp_offer = std::str::from_utf8(&tx.original.body).unwrap_or("").to_string();
    let ssrc = tx_di_gb28181::sdp::parse_sdp_ssrc(&sdp_offer)
        .unwrap_or_else(|| "0000000001".to_string());

    info!(device_id = %device_id, call_id = %call_id, "📹 收到点播 INVITE");

    // 根据 SDP offer 中的 s= 字段判断会话类型
    let session_type = if sdp_offer.contains("s=Download") {
        tx_di_gb28181::sdp::SessionType::Download
    } else if sdp_offer.contains("s=Playback") {
        tx_di_gb28181::sdp::SessionType::Playback
    } else {
        tx_di_gb28181::sdp::SessionType::Play
    };
    let is_realtime = matches!(session_type, tx_di_gb28181::sdp::SessionType::Play);

    // 回放或下载时：从 offer 的 t= 字段解析时间范围（解析失败 fallback 到 None）
    let time_range = if !is_realtime {
        let extract_t = |sdp: &str| -> Option<(u64, u64)> {
            for line in sdp.lines() {
                if let Some(rest) = line.strip_prefix("t=") {
                    let mut parts = rest.split_whitespace();
                    let start: u64 = parts.next()?.parse().ok()?;
                    let end: u64 = parts.next()?.parse().ok()?;
                    if start > 0 {
                        return Some((start, end));
                    }
                }
            }
            None
        };
        extract_t(&sdp_offer)
    } else {
        None
    };

    let sdp_answer = tx_di_gb28181::sdp::build_sdp_answer(
        "127.0.0.1", 0, &ssrc, &device_id, session_type, time_range, None,
    )
    .unwrap_or_default();

    let endpoint_inner = tx.endpoint_inner.clone();
    let dialog_layer = Arc::new(DialogLayer::new(endpoint_inner));
    let (state_tx, mut state_rx) = dialog_layer.new_dialog_state_channel();

    match dialog_layer.get_or_create_server_invite(&tx, state_tx, None, None) {
        Ok(server_dialog) => {
            let ct = rsip::Header::ContentType("application/sdp".into());
            if let Err(e) = server_dialog.accept(Some(vec![ct]), Some(sdp_answer.into_bytes())) {
                warn!(error = %e, "回复 200 OK 失败");
                return;
            }
            info!(call_id = %call_id, "✅ 点播 200 OK 已发送");
            let channel_id = ssrc.clone();
            mgr.emit(DeviceEvent::InviteReceived {
                device_id: device_id.clone(), channel_id, call_id: call_id.clone(),
            });
            let mgr_c = mgr.clone();
            let did = device_id.clone();
            tokio::spawn(async move {
                while let Some(state) = state_rx.recv().await {
                    if let rsipstack::dialog::dialog::DialogState::Terminated(_, _) = state {
                        mgr_c.emit(DeviceEvent::InviteEnded {
                            device_id: did.clone(), call_id: call_id.clone(),
                        });
                        break;
                    }
                }
            });
        }
        Err(e) => warn!(error = %e, "创建服务端对话失败"),
    }
}

async fn handle_bye(mgr: Arc<DeviceManager>, device_id: String, mut tx: Transaction) {
    let call_id = tx.original.call_id_header()
        .map(|h| h.value().to_string())
        .unwrap_or_else(|_| "N/A".into());
    let _ = tx.reply(StatusCode::OK).await;
    mgr.emit(DeviceEvent::InviteEnded { device_id, call_id });
}

async fn handle_message(
    mgr: Arc<DeviceManager>,
    device_id: String,
    endpoint: rsipstack::transaction::endpoint::EndpointInnerRef,
    config: Arc<GbCamsConfig>,
    dev: VirtualDevice,
    mut tx: Transaction,
) {
    let body = std::str::from_utf8(&tx.original.body).unwrap_or("").to_string();
    let _ = tx.reply(StatusCode::OK).await;
    if body.is_empty() { return; }

    let cmd_type = match tx_di_gb28181::xml::parse_xml_field(&body, "CmdType") {
        Some(c) => c, None => return,
    };
    let sn = tx_di_gb28181::xml::parse_sn(&body);

    match cmd_type.as_str() {
        "Catalog" => {
            info!(device_id = %device_id, sn = sn, "📂 收到目录查询");
            mgr.emit(DeviceEvent::CatalogQuery { device_id: device_id.clone(), sn });
            let cfg = config.clone();
            let dev_c = dev.clone();
            let did = device_id.clone();
            let ep = endpoint.clone();
            tokio::spawn(async move {
                let ch_pairs: Vec<(String, String)> = dev_c.channels.iter()
                    .map(|c| (c.channel_id.clone(), c.name.clone())).collect();
                let xml = tx_di_gb28181::xml::build_catalog_response_xml(&did, sn, &ch_pairs);
                let _ = send_manscdp(&ep, &cfg, &dev_c, &xml, sn).await;
            });
        }
        "DeviceInfo" => {
            let cfg = config.clone();
            let dev_c = dev.clone();
            let did = device_id.clone();
            let ep = endpoint.clone();
            tokio::spawn(async move {
                let xml = format!(
                    "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n<Response>\
                     <CmdType>DeviceInfo</CmdType><SN>{sn}</SN><DeviceID>{did}</DeviceID>\
                     <Result>OK</Result><BasicParam><Name>{name}</Name>\
                     <Manufacturer>tx</Manufacturer><Model>SimIPC</Model>\
                     <Firmware>1.0.0</Firmware></BasicParam></Response>",
                    sn=sn, did=did, name=dev_c.name
                );
                let _ = send_manscdp(&ep, &cfg, &dev_c, &xml, sn).await;
            });
        }
        "DeviceStatus" => {
            let cfg = config.clone();
            let dev_c = dev.clone();
            let did = device_id.clone();
            let ep = endpoint.clone();
            tokio::spawn(async move {
                let xml = format!(
                    "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n<Response>\
                     <CmdType>DeviceStatus</CmdType><SN>{sn}</SN><DeviceID>{did}</DeviceID>\
                     <Result>OK</Result><Online>ON</Online><Status>OK</Status>\
                     <Encode>ON</Encode><Record>OFF</Record></Response>",
                    sn=sn, did=did
                );
                let _ = send_manscdp(&ep, &cfg, &dev_c, &xml, sn).await;
            });
        }
        "RecordInfo" => {
            let cfg = config.clone();
            let dev_c = dev.clone();
            let did = device_id.clone();
            let ep = endpoint.clone();
            tokio::spawn(async move {
                let xml = format!(
                    "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n<Response>\
                     <CmdType>RecordInfo</CmdType><SN>{sn}</SN><DeviceID>{did}</DeviceID>\
                     <SumNum>0</SumNum></Response>",
                    sn=sn, did=did
                );
                let _ = send_manscdp(&ep, &cfg, &dev_c, &xml, sn).await;
            });
        }
        "Keepalive" => {}
        other => warn!(device_id = %device_id, cmd = %other, "未处理的平台指令"),
    }
}

// ── 通用 MESSAGE 发送 ─────────────────────────────────────────────────────────

async fn send_manscdp(
    endpoint: &rsipstack::transaction::endpoint::EndpointInnerRef,
    config: &GbCamsConfig,
    dev: &VirtualDevice,
    body: &str,
    seq: u32,
) -> anyhow::Result<()> {
    let req_uri = rsip::Uri::try_from(config.platform_uri().as_str())
        .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;
    let from_str = format!("sip:{}@127.0.0.1:{}", dev.device_id, dev.sip_port);
    let from_uri = rsip::Uri::try_from(from_str.as_str())
        .map_err(|e| anyhow::anyhow!("无效的 From URI: {}", e))?;
    let via = endpoint.get_via(None, None)
        .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {}", e))?;

    let from = rsip::typed::From {
        display_name: None,
        uri: from_uri.into(),
        params: vec![rsip::Param::Tag(rsip::uri::Tag::new(rsipstack::transaction::make_tag()))],
    };
    let to = rsip::typed::To {
        display_name: None,
        uri: req_uri.clone().into(),
        params: vec![],
    };

    let mut request = endpoint.make_request(
        rsip::method::Method::Message, req_uri, via, from, to, seq, None,
    );
    request.headers.push(rsip::Header::ContentType("Application/MANSCDP+xml".into()));
    request.body = body.as_bytes().to_vec().into();

    let key = TransactionKey::from_request(&request, TransactionRole::Client)
        .map_err(|e| anyhow::anyhow!("构造事务 key 失败: {}", e))?;
    let mut tx = Transaction::new_client(key, request, endpoint.clone(), None);
    tx.send().await.map_err(|e| anyhow::anyhow!("发送 MESSAGE 失败: {}", e))?;
    Ok(())
}

fn set_device_status(mgr: &DeviceManager, device_id: &str, status: DeviceStatus, error: Option<String>) {
    if let Some(mut dev) = mgr.devices.get_mut(device_id) {
        dev.status = status;
        dev.error = error;
    }
}
