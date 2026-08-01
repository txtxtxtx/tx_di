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
use tx_gb28181::GbVersion;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use tx_di_core::RIE;
use tx_di_sip::{SipClient, SipClientConfig, SipPlugin};

/// 下级平台级联管理器
///
/// **注册/续期/重连/注销生命周期由 [`tx_di_sip::SipClient`] 统一管理**：
/// 本模块只负责「配置 SipClient + 周期目录推送回调」。
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
    /// 通用注册客户端（注册生命周期托管）
    sip_client: Arc<SipClient>,
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
        sip_client: Arc<SipClient>,
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
            sip_client,
            device_registry,
            seq: AtomicU32::new(1),
            upper_version: config.upper_version,
            registered: AtomicBool::new(false),
        })
    }

    /// 启动级联：配置 SipClient（注册 + 周期目录推送）
    pub fn start(self, _cancel_token: CancellationToken) {
        let upper = self.upper_sip.clone();
        let local_id = self.local_platform_id.clone();
        let pwd = self.auth_password.clone();
        let expires = self.expires;
        // 提前取 Arc<SipClient>（后续 self 被 Arc::new 移动）
        let sip_client = self.sip_client.clone();

        info!(
            upper = %upper,
            upper_id = %self.upper_platform_id,
            "🔗 下级平台级联任务启动（注册托管 SipClient）"
        );

        // 1) 注册参数注入 SipClient（优先于 [sip_client] TOML）
        let reg_cfg = SipClientConfig {
            registrar: upper,
            username: local_id,
            password: pwd,
            realm: None,
            expires,
            renew_secs: 0,
            // 目录推送间隔 = expires/2（与原循环一致）
            keepalive_secs: (expires / 2).max(30),
            max_retries: 5,
            backoff_base_secs: 2,
            enabled: true,
        };
        if let Err(e) = sip_client.set_registration(reg_cfg) {
            error!(error = %e, "级联：注册参数注入 SipClient 失败");
            return;
        }

        // 2) 周期回调：推送目录（注册成功后由 SipClient 周期触发）
        let cascade = Arc::new(self);
        let c = cascade.clone();
        if let Err(e) = sip_client.on_keepalive(move || {
            let c = c.clone();
            async move { c.push_catalog().await }
        }) {
            error!(error = %e, "级联：注册 keepalive 回调失败");
            return;
        }

        // 3) 注册状态同步（registered 标志 + 日志）
        let c = cascade.clone();
        let _ = sip_client.on_registered(move |ok: bool| {
            c.registered.store(ok, Ordering::Relaxed);
            if ok {
                info!("✅ 下级平台注册到上级成功");
            } else {
                warn!("下级平台注册失败/断开");
            }
        });

        // 注意：`cascade` 的 Arc 引用由回调闭包持有（SipClient 的 hook 存活期间对象存活）
    }

    /// 推送设备目录到上级平台（按上级版本编码出网 XML）
    ///
    /// 由 SipClient 的 keepalive 回调周期触发（注册成功后）。
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

