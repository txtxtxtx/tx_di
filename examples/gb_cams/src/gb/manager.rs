//! GB28181 业务管理器
//!
//! 负责：注册、刷新注册、注销、发送心跳 MESSAGE、发送目录。

use super::config::Gb28181Config;
use rsipstack::dialog::authenticate::Credential;
use rsipstack::dialog::registration::Registration;
use rsipstack::sip::{self as rsip, StatusCode};
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::Transaction;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use tx_di_sip::SipPlugin;

/// GB28181 设备管理器
pub struct Gb28181Manager {
    config: Arc<Gb28181Config>,
    /// 消息序列号（心跳 SN 自增）
    sn: AtomicU32,
}

impl Gb28181Manager {
    pub fn new(config: Arc<Gb28181Config>) -> Self {
        Self {
            config,
            sn: AtomicU32::new(1),
        }
    }

    // ── 注册 ─────────────────────────────────────────────────────────────────

    /// 向上级平台注册
    ///
    /// 流程：
    /// 1. 发送 `REGISTER`（无认证头）
    /// 2. 收到 `401 Unauthorized`（rsipstack 自动捕获 challenge）
    /// 3. 使用 MD5 摘要重新发送 `REGISTER`
    /// 4. 收到 `200 OK`，注册成功
    pub async fn register(&self) -> anyhow::Result<()> {
        let sender = SipPlugin::sender();

        info!(
            platform = %self.config.platform_uri(),
            username = %self.config.username,
            "📡 开始 GB28181 注册..."
        );

        let resp = sender
            .register(
                &self.config.platform_uri(),
                &self.config.username,
                &self.config.password,
            )
            .await?;

        if resp.status_code == StatusCode::OK {
            info!("✅ 注册成功！平台: {}", self.config.platform_uri());
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "注册失败，服务器响应: {}",
                resp.status_code
            ))
        }
    }

    /// 注销（发送 Expires: 0 的 REGISTER）
    ///
    /// GB28181 规范：设备下线前应主动注销，`Expires: 0` 表示注销。
    pub async fn unregister(&self) -> anyhow::Result<()> {
        let sender = SipPlugin::sender();
        let inner = sender.inner();

        info!("📡 注销中...");

        let credential = Credential {
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            realm: Some(self.config.realm.clone()),
        };

        let server_uri = rsip::Uri::try_from(self.config.platform_uri().as_str())
            .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

        let mut reg = Registration::new(inner, Some(credential));
        // expires = Some(0) 表示注销
        let resp = reg
            .register(server_uri, Some(0))
            .await
            .map_err(|e| anyhow::anyhow!("注销失败: {}", e))?;

        info!("注销响应: {}", resp.status_code);
        Ok(())
    }

    // ── 心跳 ─────────────────────────────────────────────────────────────────

    /// 发送单次 GB28181 心跳 MESSAGE
    ///
    /// GB28181 心跳规范：
    /// - 方法：`MESSAGE`
    /// - Content-Type: `Application/MANSCDP+xml`
    /// - Body：`<CmdType>Keepalive</CmdType>` XML
    ///
    /// 心跳失败不应终止程序，调用方捕获 warn 即可。
    pub async fn send_keepalive(&self) -> anyhow::Result<()> {
        let sn = self.sn.fetch_add(1, Ordering::Relaxed);
        let sender = SipPlugin::sender();
        let inner = sender.inner();

        let platform_uri_str = self.config.platform_uri();
        let device_id = &self.config.device_id;
        let local_ip = self.config.local_ip.as_deref().unwrap_or("192.168.1.100");
        let local_port = self.config.local_port.unwrap_or(5060);

        // 构造心跳 XML body
        let xml_body = format!(
            "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
             <Notify>\r\n\
             <CmdType>Keepalive</CmdType>\r\n\
             <SN>{sn}</SN>\r\n\
             <DeviceID>{device_id}</DeviceID>\r\n\
             <Status>OK</Status>\r\n\
             </Notify>",
            sn = sn,
            device_id = device_id
        );

        // 解析 URI
        let req_uri = rsip::Uri::try_from(platform_uri_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的平台 URI '{}': {}", platform_uri_str, e))?;

        let from_str = format!("sip:{}@{}:{}", device_id, local_ip, local_port);
        let from_uri = rsip::Uri::try_from(from_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的 From URI: {}", e))?;

        // 从 EndpointInner 获取 Via 头（rsipstack 自动填入本机传输地址）
        let via = inner
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {}", e))?;

        // 构造 From / To
        let from = rsip::typed::From {
            display_name: None,
            uri: from_uri.clone().into(),
            params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
                rsipstack::transaction::make_tag(),
            ))],
        };
        let to = rsip::typed::To {
            display_name: None,
            uri: req_uri.clone().into(),
            params: vec![],
        };

        // 通过 EndpointInner 构造 SIP MESSAGE 请求
        let mut request = inner.make_request(
            rsip::method::Method::Message,
            req_uri.clone(),
            via,
            from,
            to,
            sn,
            None,
        );

        // 添加 Content-Type 和 Body
        request
            .headers
            .push(rsip::Header::ContentType("Application/MANSCDP+xml".into()));
        request.body = xml_body.into_bytes().into();

        // 创建客户端事务并发送
        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("构造事务 key 失败: {}", e))?;

        let mut tx = Transaction::new_client(key, request, inner, None);
        tx.send()
            .await
            .map_err(|e| anyhow::anyhow!("发送心跳失败: {}", e))?;

        info!(sn = sn, "💓 心跳发送成功 (MESSAGE Keepalive)");
        Ok(())
    }

    // ── 目录上报 ──────────────────────────────────────────────────────────────

    /// 响应目录查询，向平台发送设备目录
    ///
    /// GB28181 目录规范：
    /// - 方法：`MESSAGE`
    /// - CmdType：`Catalog`
    /// - 每次响应一批 `DeviceList`
    pub async fn send_catalog_response(&self, query_sn: u32) -> anyhow::Result<()> {
        let sn = query_sn; // 目录响应 SN 应与查询的 SN 一致
        let device_id = &self.config.device_id;

        // 子通道编号（末尾 00 为通道）
        let channel_id = format!("{}00", &device_id[..18]);
        let xml_body = format!(
            "<?xml version=\"1.0\" encoding=\"GB2312\"?>\r\n\
             <Response>\r\n\
             <CmdType>Catalog</CmdType>\r\n\
             <SN>{sn}</SN>\r\n\
             <DeviceID>{device_id}</DeviceID>\r\n\
             <SumNum>1</SumNum>\r\n\
             <DeviceList Num=\"1\">\r\n\
             <Item>\r\n  <DeviceID>{channel_id}</DeviceID>\r\n\
               <Name>Camera-01</Name>\r\n\
               <Manufacturer>Simulator</Manufacturer>\r\n\
               <Model>IPC-V1</Model>\r\n\
               <Owner>Owner</Owner>\r\n\
               <CivilCode>3402000000</CivilCode>\r\n\
               <Address>模拟摄像头</Address>\r\n\
               <Parental>0</Parental>\r\n\
               <ParentID>{device_id}</ParentID>\r\n\
               <SafetyWay>0</SafetyWay>\r\n\
               <RegisterWay>1</RegisterWay>\r\n\
               <Secrecy>0</Secrecy>\r\n\
               <Status>ON</Status>\r\n\
             </Item>\r\n\
             </DeviceList>\r\n\
             </Response>",
            sn = sn,
            device_id = device_id,
            channel_id = channel_id
        );

        info!(sn = sn, "📂 发送目录响应");

        // 发送逻辑与 send_keepalive 相同（复用）
        self.send_message_to_platform(&xml_body, "Application/MANSCDP+xml", sn)
            .await
    }

    // ── 主循环 ────────────────────────────────────────────────────────────────

    /// 注册 + 心跳主循环
    ///
    /// 1. 注册成功后启动心跳定时器
    /// 2. 每隔 `heartbeat_secs` 发送一次 Keepalive
    /// 3. 每隔 `register_ttl / 2` 刷新注册（防止过期）
    /// 4. 收到 Ctrl+C 信号退出
    pub async fn run(&self) -> anyhow::Result<()> {
        // 首次注册
        self.register().await?;

        let heartbeat_interval = Duration::from_secs(self.config.heartbeat_secs);
        // 刷新注册：在 TTL 一半时刷新，避免超时
        let refresh_interval =
            Duration::from_secs((self.config.register_ttl / 2).max(60) as u64);

        let mut heartbeat_ticker = interval(heartbeat_interval);
        let mut refresh_ticker = interval(refresh_interval);
        // 跳过首次立即触发（已手动注册）
        heartbeat_ticker.tick().await;
        refresh_ticker.tick().await;

        info!(
            heartbeat_secs = self.config.heartbeat_secs,
            refresh_secs = self.config.register_ttl / 2,
            "⏰ 心跳 & 刷新注册定时器已启动"
        );

        loop {
            tokio::select! {
                // Ctrl+C 信号
                _ = tokio::signal::ctrl_c() => {
                    info!("收到退出信号，准备注销...");
                    break;
                }
                // 心跳
                _ = heartbeat_ticker.tick() => {
                    if let Err(e) = self.send_keepalive().await {
                        warn!("心跳发送失败: {}（将在下次重试）", e);
                    }
                }
                // 刷新注册
                _ = refresh_ticker.tick() => {
                    info!("🔄 刷新注册（防止 TTL 超时）");
                    if let Err(e) = self.register().await {
                        error!("刷新注册失败: {}（将在下次重试）", e);
                    }
                }
            }
        }

        Ok(())
    }

    // ── 内部工具 ──────────────────────────────────────────────────────────────

    /// 向平台发送 SIP MESSAGE 的通用方法
    async fn send_message_to_platform(
        &self,
        body: &str,
        content_type: &str,
        seq: u32,
    ) -> anyhow::Result<()> {
        let sender = SipPlugin::sender();
        let inner = sender.inner();

        let platform_uri_str = self.config.platform_uri();
        let device_id = &self.config.device_id;
        let local_ip = self.config.local_ip.as_deref().unwrap_or("192.168.1.100");
        let local_port = self.config.local_port.unwrap_or(5060);

        let req_uri = rsip::Uri::try_from(platform_uri_str.as_str())
            .map_err(|e| anyhow::anyhow!("无效的平台 URI: {}", e))?;

        let from_str = format!("sip:{}@{}:{}", device_id, local_ip, local_port);
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
