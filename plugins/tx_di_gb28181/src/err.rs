//! GB28181 插件错误码

use tx_error::CodeMsg;

/// GB28181 插件业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("GB")]
#[ie(tx_di_core::IE)]
pub enum GbErr {
    /// 设备未注册或已离线
    #[err(-1, "设备未注册或已离线")]
    DeviceNotFound,
    /// 无效的 SIP URI
    #[err(-2, "无效的 SIP URI")]
    InvalidUri,
    /// INVITE 呼叫失败
    #[err(-3, "INVITE 呼叫失败")]
    InviteFailed,
    /// 级联注册失败
    #[err(-4, "级联注册失败")]
    RegisterFailed,
    /// 注销失败
    #[err(-5, "注销失败")]
    UnregisterFailed,
    /// MESSAGE 发送失败
    #[err(-6, "MESSAGE 发送失败")]
    MessageSendFailed,
    /// 媒体 API 请求失败
    #[err(-7, "媒体 API 请求失败")]
    MediaApiRequestFailed,
    /// 媒体 API 返回错误
    #[err(-8, "媒体 API 返回错误")]
    MediaApiError,
    /// 媒体 API 响应格式无效
    #[err(-9, "媒体 API 响应格式无效")]
    MediaApiResponseInvalid,
    /// RTP 端口分配失败
    #[err(-10, "RTP 端口分配失败")]
    RtpPortFailed,
    /// 无可用端口
    #[err(-11, "无可用端口")]
    NoAvailablePort,
    /// 不支持的操作
    #[err(-12, "不支持的操作")]
    UnsupportedOperation,
}
