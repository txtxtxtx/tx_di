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
use crate::err::GbErr;
use rsipstack::sip as rsip;
use rsipstack::sip::{SipMessage, StatusCode};
use rsipstack::transaction::key::{TransactionKey, TransactionRole};
use rsipstack::transaction::transaction::{Transaction, TransactionEvent};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use tx_di_core::RIE;
use tx_di_sip::SipPlugin;
use tx_gb28181::utils::{md5_digest, md5_hex};

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
    /// 上次 401 nonce（用于重试）
    last_nonce: std::sync::Mutex<Option<String>>,
    /// 当前注册状态
    registered: std::sync::atomic::AtomicBool,
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
            last_nonce: std::sync::Mutex::new(None),
            registered: std::sync::atomic::AtomicBool::new(false),
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

    /// 向上级平台发送 REGISTER（含摘要认证）
    async fn register(&self) -> RIE<()> {
        let sender = self.sip_plugin.sender()?;
        let inner = sender.inner();

        let req_uri_str = &self.upper_sip;
        let req_uri = rsip::Uri::try_from(req_uri_str.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let from_str = format!("sip:{}@{}", self.local_platform_id, self.local_sip_ip);
        let from_uri = rsip::Uri::try_from(from_str.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let to_str = format!("sip:{}@{}", self.upper_platform_id, self.local_sip_ip);
        let to_uri = rsip::Uri::try_from(to_str.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let via = inner
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {e}"))?;

        let from_header = rsip::typed::From {
            display_name: None,
            uri: from_uri.clone(),
            params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
                rsipstack::transaction::make_tag(),
            ))],
        };

        // make_request 内部自动生成 Call-Id，传 None 即可
        let mut request = inner.make_request(
            rsip::method::Method::Register,
            req_uri.clone(),
            via.clone(),
            from_header.clone(),
            rsip::typed::To {
                display_name: None,
                uri: to_uri.clone(),
                params: vec![],
            },
            self.seq.fetch_add(1, Ordering::Relaxed),
            None,
        );

        request
            .headers
            .push(rsip::Header::Contact(
                rsip::headers::untyped::Contact::new(from_uri.to_string()),
            ));
        request
            .headers
            .push(rsip::Header::Expires(self.expires.into()));

        // 如果之前收到过 401，加上 Authorization
        if let Some(nonce) = self.last_nonce.lock().unwrap().as_ref() {
            let auth_value = build_digest_auth(
                &self.local_platform_id,
                &self.auth_password,
                "REGISTER",
                req_uri_str,
                nonce,
            );
            request
                .headers
                .push(rsip::Header::Authorization(
                    rsip::headers::untyped::Authorization::new(auth_value),
                ));
        }

        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("构造 REGISTER 事务 key 失败: {e}"))?;

        // 通过 Transaction 内置 TU channel 接收响应
        let mut tx = Transaction::new_client(key, request, inner, None);
        tx.send()
            .await
            .map_err(|_| GbErr::RegisterFailed)?;

        // 等待响应（超时 5 秒）
        match tokio::time::timeout(Duration::from_secs(5), tx.tu_receiver.recv()).await {
            Ok(Some(TransactionEvent::Received(SipMessage::Response(response), _))) => {
                let status = response.status_code;
                match status {
                    StatusCode::OK => {
                        self.registered.store(true, Ordering::Relaxed);
                        info!("✅ 下级平台注册到上级成功");
                    }
                    StatusCode::Unauthorized => {
                        // 提取 nonce 并重试
                        let www_auth = response
                            .headers
                            .iter()
                            .find_map(|h| {
                                let s = format!("{h}");
                                if s.to_lowercase().starts_with("www-authenticate:") {
                                    Some(s)
                                } else {
                                    None
                                }
                            });

                        if let Some(ref auth_str) = www_auth {
                            let nonce = extract_nonce(auth_str);
                            if !nonce.is_empty() {
                                debug!(nonce = %nonce, "收到 401，提取 nonce 准备重试");
                                *self.last_nonce.lock().unwrap() = Some(nonce);
                                // 立即重试（带 Authorization）
                                drop(tx);
                                match Box::pin(self.register()).await {
                                    Err(e) => {
                                        warn!(error = %e, "带认证重试注册失败");
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {
                        warn!(
                            status = %status,
                            "上级平台返回非预期状态码"
                        );
                    }
                }
            }
            Ok(Some(TransactionEvent::Terminate(_))) => {
                warn!("REGISTER 事务被终止");
            }
            _ => {
                warn!("REGISTER 未收到响应（超时）");
            }
        }

        Ok(())
    }

    /// 向上级平台注销（Expires: 0）
    async fn unregister(&self) -> RIE<()> {
        let sender = self.sip_plugin.sender()?;
        let inner = sender.inner();

        let req_uri = rsip::Uri::try_from(self.upper_sip.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let from_str = format!("sip:{}@{}", self.local_platform_id, self.local_sip_ip);
        let from_uri = rsip::Uri::try_from(from_str.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let via = inner
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {e}"))?;

        // make_request 内部自动生成 Call-Id
        let mut request = inner.make_request(
            rsip::method::Method::Register,
            req_uri.clone(),
            via,
            rsip::typed::From {
                display_name: None,
                uri: from_uri.clone(),
                params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
                    rsipstack::transaction::make_tag(),
                ))],
            },
            rsip::typed::To {
                display_name: None,
                uri: req_uri,
                params: vec![],
            },
            self.seq.fetch_add(1, Ordering::Relaxed),
            None,
        );

        request
            .headers
            .push(rsip::Header::Contact(
                rsip::headers::untyped::Contact::new(from_uri.to_string()),
            ));
        request.headers.push(rsip::Header::Expires(0.into()));

        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("构造注销事务 key 失败: {e}"))?;

        let mut tx = Transaction::new_client(key, request, inner, None);
        tx.send()
            .await
            .map_err(|_| GbErr::UnregisterFailed)?;

        info!("下级平台已向上级注销");
        Ok(())
    }

    /// 推送设备目录到上级平台
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

        self.send_msg(&body, sn).await
    }

    /// 发送 SIP MESSAGE 到上级平台
    async fn send_msg(&self, body: &str, seq: u32) -> RIE<()> {
        let sender = self.sip_plugin.sender()?;
        let inner = sender.inner();

        let req_uri = rsip::Uri::try_from(self.upper_sip.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let from_str = format!("sip:{}@{}", self.local_platform_id, self.local_sip_ip);
        let from_uri = rsip::Uri::try_from(from_str.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let via = inner
            .get_via(None, None)
            .map_err(|e| anyhow::anyhow!("获取 Via 头失败: {e}"))?;

        let from = rsip::typed::From {
            display_name: None,
            uri: from_uri,
            params: vec![rsip::Param::Tag(rsip::uri::Tag::new(
                rsipstack::transaction::make_tag(),
            ))],
        };

        let to_uri = rsip::Uri::try_from(self.upper_sip.as_str())
            .map_err(|_| GbErr::InvalidUri)?;

        let mut request = inner.make_request(
            rsip::method::Method::Message,
            req_uri,
            via,
            from,
            rsip::typed::To {
                display_name: None,
                uri: to_uri,
                params: vec![],
            },
            seq,
            None,
        );

        request
            .headers
            .push(rsip::Header::ContentType(
                "Application/MANSCDP+xml".into(),
            ));
        request.body = body.as_bytes().to_vec();

        let key = TransactionKey::from_request(&request, TransactionRole::Client)
            .map_err(|e| anyhow::anyhow!("构造 MESSAGE 事务 key 失败: {e}"))?;

        let mut tx = Transaction::new_client(key, request, inner, None);
        tx.send()
            .await
            .map_err(|_| GbErr::MessageSendFailed)?;

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

// ── 摘要认证辅助函数 ─────────────────────────────────────────────────────────

/// 构建 Digest Authorization 头值
fn build_digest_auth(
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    nonce: &str,
) -> String {
    let realm = "3402000000";
    let ha1 = md5_hex(md5_digest(format!("{username}:{realm}:{password}").as_bytes()));
    let ha2 = md5_hex(md5_digest(format!("{method}:{uri}").as_bytes()));
    let response = md5_hex(md5_digest(format!("{ha1}:{nonce}:{ha2}").as_bytes()));

    format!(
        "Digest username=\"{username}\", realm=\"{realm}\", nonce=\"{nonce}\", uri=\"{uri}\", response=\"{response}\", algorithm=MD5"
    )
}

/// 从 WWW-Authenticate 头中提取 nonce
fn extract_nonce(auth_str: &str) -> String {
    auth_str
        .split("nonce=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_digest_auth() {
        let auth = build_digest_auth(
            "34020000002000000001",
            "12345678",
            "REGISTER",
            "sip:34020000002000000002@192.168.1.1:5060",
            "abc123",
        );
        assert!(auth.starts_with("Digest "));
        assert!(auth.contains("username=\"34020000002000000001\""));
        assert!(auth.contains("nonce=\"abc123\""));
        assert!(auth.contains("algorithm=MD5"));
    }

    #[test]
    fn test_extract_nonce() {
        let header = "Digest realm=\"3402000000\", nonce=\"def456\", algorithm=MD5";
        assert_eq!(extract_nonce(header), "def456");
    }
}
