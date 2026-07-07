//! GB28181-2022 级联（上下级平台互联）
//!
//! 实现 GB28181 标准的平台间级联互联功能，支持下级平台向上级注册。
//!
//! ## 下级模式 (enable_lower = true)
//!
//! ```text
//! 本平台 ── REGISTER ──→ 上级平台（含摘要认证）
//! 本平台 ── MESSAGE(Keepalive) ──→ 上级平台（心跳）
//! 本平台 ── MESSAGE(Catalog) ──→ 上级平台（目录推送）
//! ```
//!
//! ## 上级模式 (enable_upper = true)
//!
//! 本平台作为上级时，天然接收下级/设备的 REGISTER 请求（由现有 handlers.rs 处理）。
//! 需要在 REGISTER 处理中增加来源类型标记（直连设备 vs 下级平台）。

use crate::config::CascadeConfig;
use crate::device_registry::DeviceRegistry;
use rsipstack::sip::StatusCode;
use tx_gb28181::GbVersion;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use tx_di_core::RIE;
use tx_di_sip::SipPlugin;

/// 下级平台级联管理器
pub struct CascadeLower {
    /// 上级平台 SIP URI（如 "sip:192.168.1.1:5060"）
    upper_sip: String,
    /// 上级平台 ID
    upper_platform_id: String,
    /// 本平台 ID
    local_platform_id: String,
    /// 本平台 SIP IP
    local_sip_ip: String,
    /// 认证密码
    auth_password: String,
    /// 注册有效期
    expires: u32,
    /// SIP 插件引用
    sip_plugin: Arc<SipPlugin>,
    /// 设备注册表（用于构建目录）
    device_registry: DeviceRegistry,
    /// 序列号
    seq: AtomicU32,
    /// 上级平台协议版本（决定出网 XML 字符集与指令集）
    upper_version: GbVersion,
    /// 当前注册状态
    registered: AtomicBool,
}

impl CascadeLower {
    /// 创建下级平台级联管理器
    pub fn new(
        config: &CascadeConfig,
        platform_id: &str,
        sip_ip: &str,
        sip_plugin: Arc<SipPlugin>,
        device_registry: DeviceRegistry,
    ) -> Option<Self> {
        let upper_sip = config.upper_platform_sip.as_ref()?;
        let upper_platform_id = config.upper_platform_id.as_ref()?;
        let auth_password = config
            .upper_auth_password
            .clone()
            .unwrap_or_else(|| "12345678".to_string());

        Some(Self {
            upper_sip: upper_sip.clone(),
            upper_platform_id: upper_platform_id.clone(),
            local_platform_id: platform_id.to_string(),
            local_sip_ip: sip_ip.to_string(),
            auth_password,
            expires: 3600,
            sip_plugin,
            device_registry,
            seq: AtomicU32::new(1),
            upper_version: config.upper_version,
            registered: AtomicBool::new(false),
        })
    }

    /// 启动级联后台任务
    pub fn start(self, cancel_token: CancellationToken) {
        tokio::spawn(async move {
            if let Err(e) = self.run(cancel_token).await {
                error!(error = %e, "下级平台级联任务异常退出");
            }
        });
    }

    /// 主循环
    async fn run(self, cancel_token: CancellationToken) -> RIE<()> {
        info!(
            upper = %self.upper_sip,
            upper_id = %self.upper_platform_id,
            "🔗 下级平台级联任务启动"
        );

        // 首次注册
        if let Err(e) = self.register().await {
            warn!(error = %e, "首次级联注册失败，将在下一个周期重试");
        }

        // 定期续约（expires/2 间隔）
        let renew_interval = Duration::from_secs((self.expires / 2).max(30) as u64);
        let mut ticker = interval(renew_interval);
        ticker.tick().await; // 跳过立即触发

        loop {
            tokio::select! {
                biased;
                _ = cancel_token.cancelled() => {
                    // 发送注销
                    if self.registered.load(Ordering::Relaxed) {
                        let _ = self.unregister().await;
                    }
                    info!("下级平台级联任务已停止");
                    return Ok(());
                }
                _ = ticker.tick() => {
                    if let Err(e) = self.register().await {
                        warn!(error = %e, "级联注册续约失败");
                    } else {
                        // 注册成功后推送目录
                        if let Err(e) = self.push_catalog().await {
                            warn!(error = %e, "级联目录推送失败");
                        }
                    }
                }
            }
        }
    }

    /// 向上级平台发送 REGISTER（复用 `SipSender::register` 的自动 401 重认证）
    async fn register(&self) -> RIE<()> {
        let sender = self.sip_plugin.sender()?;
        let resp = sender
            .register(&self.upper_sip, &self.local_platform_id, &self.auth_password)
            .await?;

        match resp.status_code {
            StatusCode::OK => {
                self.registered.store(true, Ordering::Relaxed);
                info!("✅ 下级平台注册到上级成功");
            }
            _ => {
                warn!(status = %resp.status_code, "上级平台返回非预期状态码");
            }
        }
        Ok(())
    }

    /// 向上级平台注销（Expires: 0，复用 `SipSender::unregister`）
    async fn unregister(&self) -> RIE<()> {
        let sender = self.sip_plugin.sender()?;
        sender
            .unregister(&self.upper_sip, &self.local_platform_id, &self.auth_password)
            .await?;
        self.registered.store(false, Ordering::Relaxed);
        info!("下级平台已向上级注销");
        Ok(())
    }

    /// 推送设备目录到上级平台（按上级版本编码出网 XML）
    async fn push_catalog(&self) -> RIE<()> {
        let devices = self.device_registry.all_devices();
        if devices.is_empty() {
            debug!("无在线设备，跳过目录推送");
            return Ok(());
        }

        // 构建 Catalog XML (GB28181-2022)
        let sn = self.seq.fetch_add(1, Ordering::Relaxed);

        let items_xml: String = devices
            .iter()
            .map(|d| build_item_xml(&d.item))
            .collect::<Vec<_>>()
            .join("\r\n");

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n\
            <Notify>\r\n\
            <CmdType>Catalog</CmdType>\r\n\
            <SN>{sn}</SN>\r\n\
            <DeviceID>{platform_id}</DeviceID>\r\n\
            <SumNum>{sum}</SumNum>\r\n\
            <DeviceList Num=\"{sum}\">\r\n\
            {items}\r\n\
            </DeviceList>\r\n\
            </Notify>\r\n",
            sn = sn,
            platform_id = self.local_platform_id,
            sum = devices.len(),
            items = items_xml,
        );

        // 按上级平台协议版本重声明字符集并编码字节（2016→GB2312，2022→GB18030）
        let encoded = self.upper_version.serialize(&body);
        self.send_msg(&encoded, sn).await
    }

    /// 发送 SIP MESSAGE（字节）到上级平台
    async fn send_msg(&self, body: &[u8], seq: u32) -> RIE<()> {
        let sender = self.sip_plugin.sender()?;
        let from_str = format!("sip:{}@{}", self.local_platform_id, self.local_sip_ip);
        sender
            .send_message(&self.upper_sip, &from_str, body, "Application/MANSCDP+xml")
            .await?;
        debug!(sn = seq, "级联 MESSAGE 发送成功");
        Ok(())
    }
}

// ── ItemType XML 构建 ─────────────────────────────────────────────────────────

/// 将 ItemType 构建为 GB28181-2022 Catalog 的 <Item> XML 片段
fn build_item_xml(item: &tx_gb28181::enums::ItemType) -> String {
    let mut xml = format!(
        "<Item>\r\n\
         <DeviceID>{device_id}</DeviceID>\r\n\
         <Name>{name}</Name>\r\n\
         <Manufacturer>{manufacturer}</Manufacturer>\r\n\
         <Model>{model}</Model>\r\n\
         <Status>{status}</Status>\r\n\
         <Parental>{parental}</Parental>\r\n\
         <ParentID>{parent_id}</ParentID>\r\n\
         <SafetyWay>0</SafetyWay>\r\n\
         <RegisterWay>{register_way}</RegisterWay>\r\n\
         <Secrecy>{secrecy}</Secrecy>",
        device_id = item.device_id,
        name = item.name,
        manufacturer = item.manufacturer,
        model = item.model,
        status = item.status.as_str(),
        parental = item.parental,
        parent_id = item.parent_id,
        register_way = item.register_way,
        secrecy = item.secrecy,
    );

    if let Some(ref ip) = item.ip_address {
        xml.push_str(&format!("\r\n <IPAddress>{ip}</IPAddress>"));
    }
    if let Some(port) = item.port {
        xml.push_str(&format!("\r\n <Port>{port}</Port>"));
    }
    if !item.civil_code.is_empty() {
        xml.push_str(&format!("\r\n <CivilCode>{}</CivilCode>", item.civil_code));
    }
    if !item.address.is_empty() {
        xml.push_str(&format!("\r\n <Address>{}</Address>", item.address));
    }
    xml.push_str("\r\n</Item>");
    xml
}

