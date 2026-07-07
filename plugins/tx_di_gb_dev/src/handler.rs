//! 设备端业务回调 trait
//!
//! 设备只需实现 [`DeviceHandler`] 即可接入国标平台，所有方法均有默认空实现，
//! 可按需覆写。组件 [`Gb28181Device`] 在收到平台下发的查询/控制时回调对应方法。

use async_trait::async_trait;
use tx_gb28181::xml::PtzCommand;

/// 设备端业务回调 trait
///
/// 所有方法均提供默认空实现（`NoopDeviceHandler` 即全部使用默认），
/// 业务层按需覆写即可。回调由 [`crate::Gb28181Device`] 在 SIP 消息处理上下文中调用。
#[async_trait]
pub trait DeviceHandler: Send + Sync + 'static {
    /// 收到目录查询（`Catalog`），返回 `(通道ID, 名称)` 列表。
    ///
    /// 框架据此调用 [`tx_gb28181::xml::build_catalog_response_xml`] 组装响应。
    async fn on_catalog(&self, _sn: u32) -> Vec<(String, String)> {
        vec![]
    }

    /// 收到设备信息查询（`DeviceInfo`），返回完整 `Response` XML 字符串。
    async fn on_device_info(&self, _sn: u32) -> String {
        String::new()
    }

    /// 收到设备状态查询（`DeviceStatus`），返回完整 `Response` XML 字符串。
    async fn on_device_status(&self, _sn: u32) -> String {
        String::new()
    }

    /// 收到点播/语音广播 `INVITE`，返回 SDP answer 字符串。
    ///
    /// `channel_id` 取自 INVITE 的 `To` 头（平台指定的通道），
    /// `sdp_offer` 为对端 SDP offer 文本。
    async fn on_invite(&self, _channel_id: &str, _sdp_offer: &str) -> String {
        String::new()
    }

    /// 收到 `BYE`（挂断）。
    ///
    /// `call_id` 为本次会话的 SIP Call-ID，`channel_id` 为 `To` 头中指定的通道
    /// （与 `on_invite` 一致），便于精确停止该通道的媒体流。
    async fn on_bye(&self, _call_id: &str, _channel_id: &str) {}

    /// 收到 PTZ 控制（`<Control CmdType="DeviceControl">`）。
    async fn on_ptz(&self, _channel_id: &str, _cmd: &PtzCommand) {}
}

/// 默认空实现：所有回调均为 no-op，供未提供 `DeviceHandler` 时兜底。
pub struct NoopDeviceHandler;

#[async_trait]
impl DeviceHandler for NoopDeviceHandler {}
