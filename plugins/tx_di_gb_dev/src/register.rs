//! 设备端注册生命周期与 MESSAGE 处理
//!
//! - [`run_lifecycle`]：向上级平台 `REGISTER` + 周期心跳 `Keepalive`，取消时 `UNREGISTER`。
//! - [`handle_device_message`]：处理平台下发的 `MESSAGE`（目录/设备信息/状态查询/PTZ 控制），
//!   经 [`crate::handler::DeviceHandler`] 取业务数据并回网（按版本编码）。

use std::sync::Arc;
use std::time::Duration;

use rsipstack::sip::{HeadersExt, StatusCode};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tx_di_core::RIE;
use tx_di_sip::SipTx;
use tx_gb28181::Gb28181CmdType;
use tx_gb28181::sip::extract_user_from_sip_uri;
use tx_gb28181::xml::{
    build_catalog_response_xml, build_keepalive_xml, build_time_sync_response_xml,
    parse_ptz_control_xml, parse_sn, parse_xml_field,
};

use crate::config::GbDevConfig;
use crate::handler::DeviceHandler;
use crate::plugin::Gb28181Device;

/// 从 `platform_uri` 提取注册服务器地址（host:port，去掉 `sip://` 前缀与 `@` 之前部分）
fn registrar_of(cfg: &GbDevConfig) -> String {
    let s = if let Some(idx) = cfg.platform_uri.find('@') {
        &cfg.platform_uri[idx + 1..]
    } else {
        &cfg.platform_uri
    };
    s.trim_end_matches('/').to_string()
}

/// 设备自身 URI（出网 MESSAGE 的 From）
fn device_uri_of(cfg: &GbDevConfig) -> String {
    format!("sip:{}", cfg.device_id)
}

/// 从 SIP 头值中提取裸 URI（去掉 `<>` 包裹与 `;tag=...` 等参数）
fn bare_uri_of(header_value: &str) -> String {
    if let Some(start) = header_value.find('<') {
        if let Some(rel_end) = header_value[start..].find('>') {
            return header_value[start + 1..start + rel_end].to_string();
        }
    }
    header_value.to_string()
}

impl Gb28181Device {
    /// 向上级平台注册一次（复用 `SipSender::register` 的自动 401 重认证）
    pub async fn do_register(&self) -> RIE<()> {
        let sender = self.sip.sender()?;
        sender
            .register(
                &registrar_of(&self.config),
                &self.config.username,
                &self.config.password,
            )
            .await?;
        info!(device_id = %self.config.device_id, "设备注册成功");
        Ok(())
    }

    /// 向上级平台注销（Expires: 0）
    pub async fn do_unregister(&self) -> RIE<()> {
        let sender = self.sip.sender()?;
        sender
            .unregister(
                &registrar_of(&self.config),
                &self.config.username,
                &self.config.password,
            )
            .await?;
        info!(device_id = %self.config.device_id, "设备已注销");
        Ok(())
    }

    /// 发送一次心跳 Keepalive（按版本编码出网）
    pub async fn do_keepalive(&self) -> RIE<()> {
        let sender = self.sip.sender()?;
        let sn = self.next_sn();
        let xml = build_keepalive_xml(&self.config.device_id, sn);
        let encoded = self.config.version.serialize(&xml);
        let to = self.config.platform_uri.clone();
        let from = device_uri_of(&self.config);
        sender
            .send_message(&to, &from, &encoded, "Application/MANSCDP+xml")
            .await?;
        info!(device_id = %self.config.device_id, sn = sn, "心跳已发送");
        Ok(())
    }
}

/// 设备注册生命周期：注册 → 周期心跳 → 取消时注销
pub async fn run_lifecycle(dev: &Arc<Gb28181Device>, token: CancellationToken) -> RIE<()> {
    // 首次注册（失败不致命，心跳周期前会重试）
    if let Err(e) = dev.do_register().await {
        warn!(error = %e, "设备初次注册失败，将在心跳周期重试");
    }

    let dev2 = dev.clone();
    let task_token = token.clone();
    let hb = dev.config.heartbeat_secs.max(5);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(hb as u64));
        loop {
            tokio::select! {
                biased;
                _ = task_token.cancelled() => {
                    if let Err(e) = dev2.do_unregister().await {
                        warn!(error = %e, "设备注销失败");
                    }
                    info!("设备续期任务已停止");
                    return;
                }
                _ = ticker.tick() => {
                    // 周期续期注册 + 心跳
                    if let Err(e) = dev2.do_register().await {
                        warn!(error = %e, "设备续期注册失败");
                    }
                    if let Err(e) = dev2.do_keepalive().await {
                        warn!(error = %e, "设备心跳发送失败");
                    }
                }
            }
        }
    });

    token.cancelled().await;
    info!("Gb28181Device 生命周期结束");
    Ok(())
}

/// 根据指令类型，向业务回调取数据并构建待回网 XML（不含发送）
///
/// 返回 `Some(xml)` 表示需要向平台回复；`None` 表示该指令无需响应
/// （如 PTZ 控制仅触发 [`DeviceHandler::on_ptz`] 副作用）。
pub(crate) async fn build_response_xml(
    handler: &(dyn DeviceHandler + '_),
    cmd: Gb28181CmdType,
    sn: u32,
    _device_id: &str,
    body: &str,
    cfg_device_id: &str,
) -> Option<String> {
    match cmd {
        Gb28181CmdType::Catalog => {
            let channels = handler.on_catalog(sn).await;
            Some(build_catalog_response_xml(cfg_device_id, sn, &channels))
        }
        Gb28181CmdType::DeviceInfo => Some(handler.on_device_info(sn).await),
        Gb28181CmdType::DeviceStatus => {
            if body.contains("<TimeRequest>") {
                // 校时查询：直接回应当前时间
                Some(build_time_sync_response_xml(cfg_device_id, sn))
            } else {
                Some(handler.on_device_status(sn).await)
            }
        }
        Gb28181CmdType::DeviceControl => {
            if let Some((ch, ptz)) = parse_ptz_control_xml(body) {
                handler.on_ptz(&ch, &ptz).await;
            }
            None
        }
        _ => {
            warn!(cmd = %format!("{}", cmd), "设备端未处理的 MESSAGE 指令");
            None
        }
    }
}

/// 处理来自平台的 MESSAGE（查询/控制），回复相应响应
pub async fn handle_device_message(dev: &Arc<Gb28181Device>, tx: &SipTx) -> RIE<()> {
    // 先回 200 OK（GB28181 要求先确认再处理）
    tx.reply(StatusCode::OK)
        .await
        .map_err(|e| anyhow::anyhow!("回复 MESSAGE 200 OK 失败: {}", e))?;

    let body = std::str::from_utf8(&tx.request().body)
        .unwrap_or("")
        .to_string();
    if body.is_empty() {
        return Ok(());
    }

    let cmd_type = match parse_xml_field(&body, "CmdType") {
        Some(c) => c,
        None => {
            warn!("收到无 CmdType 的 MESSAGE，已忽略");
            return Ok(());
        }
    };
    let sn = parse_sn(&body);
    let device_id = extract_user_from_sip_uri(
        &tx.request()
            .from_header()
            .map(|h| h.value().to_string())
            .unwrap_or_default(),
    )
    .unwrap_or_default();

    let cmd: Gb28181CmdType = match cmd_type.parse() {
        Ok(c) => c,
        Err(_) => {
            warn!(cmd = %cmd_type, "未识别的指令类型");
            return Ok(());
        }
    };

    // 提取回网目标（平台 URI）与自身 URI
    let platform_uri = bare_uri_of(
        &tx.request()
            .from_header()
            .map(|h| h.value().to_string())
            .unwrap_or_default(),
    );
    let self_uri = bare_uri_of(
        &tx.request()
            .to_header()
            .map(|h| h.value().to_string())
            .unwrap_or_else(|_| device_uri_of(&dev.config)),
    );

    let response_xml = build_response_xml(
        &*dev.handler(),
        cmd,
        sn,
        &device_id,
        &body,
        &dev.config.device_id,
    )
    .await;

    if let Some(xml) = response_xml {
        let encoded = dev.config.version.serialize(&xml);
        let sender = dev.sip.sender()?;
        sender
            .send_message(&platform_uri, &self_uri, &encoded, "Application/MANSCDP+xml")
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct FakeHandler {
        catalog: Vec<(String, String)>,
        ptz_hits: std::sync::Arc<std::sync::atomic::AtomicU32>,
    }

    #[async_trait]
    impl DeviceHandler for FakeHandler {
        async fn on_catalog(&self, _sn: u32) -> Vec<(String, String)> {
            self.catalog.clone()
        }
        async fn on_ptz(&self, _channel_id: &str, _cmd: &tx_gb28181::xml::PtzCommand) {
            self.ptz_hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn catalog_response_uses_handler_channels() {
        let h = FakeHandler {
            catalog: vec![("34020000001320000001".into(), "前门".into())],
            ptz_hits: Default::default(),
        };
        let xml = build_response_xml(
            &h,
            Gb28181CmdType::Catalog,
            7,
            "34020000001320000001",
            "",
            "34020000001320000001",
        )
        .await
        .expect("应返回 Catalog 响应 XML");
        assert!(xml.contains("<CmdType>Catalog</CmdType>"));
        assert!(xml.contains("34020000001320000001"));
        assert!(xml.contains("前门"));
        assert!(xml.contains("<SN>7</SN>"));
    }

    #[tokio::test]
    async fn device_status_timesync_returns_time_response() {
        let h = FakeHandler {
            catalog: vec![],
            ptz_hits: Default::default(),
        };
        let body = "<Query><CmdType>DeviceStatus</CmdType><TimeRequest>2020-01-01T00:00:00</TimeRequest></Query>";
        let xml = build_response_xml(
            &h,
            Gb28181CmdType::DeviceStatus,
            3,
            "d",
            body,
            "d",
        )
        .await
        .expect("应返回校时响应 XML");
        assert!(xml.contains("<CmdType>DeviceStatus</CmdType>"));
        assert!(xml.contains("<CurrentTime>"));
    }

    #[tokio::test]
    async fn device_control_triggers_on_ptz() {
        let h = std::sync::Arc::new(FakeHandler {
            catalog: vec![],
            ptz_hits: Default::default(),
        });
        let body = "<Control><CmdType>DeviceControl</CmdType><DeviceID>ch1</DeviceID><PTZCmd>A50F01010000000001</PTZCmd></Control>";
        let resp = build_response_xml(
            &*h,
            Gb28181CmdType::DeviceControl,
            1,
            "ch1",
            body,
            "ch1",
        )
        .await;
        assert!(resp.is_none(), "PTZ 控制不应产生回网 XML");
        assert_eq!(
            h.ptz_hits.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "应触发一次 on_ptz"
        );
    }

    #[test]
    fn registrar_extraction() {
        let cfg = GbDevConfig {
            platform_uri: "sip:34020000002000000001@192.168.1.1:5060".into(),
            ..Default::default()
        };
        assert_eq!(registrar_of(&cfg), "192.168.1.1:5060");
    }
}
