//! SIP 插件错误码

use tx_error::CodeMsg;

/// SIP 插件业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("SIP")]

pub enum SipErr {
    /// 无效的 SIP 地址
    #[err(-1, "无效的 SIP 地址")]
    InvalidAddress,
    /// 无效的 SIP URI
    #[err(-2, "无效的 SIP URI")]
    InvalidUri,
    /// REGISTER 注册失败
    #[err(-3, "REGISTER 注册失败")]
    RegisterFailed,
    /// INVITE 呼叫失败
    #[err(-4, "INVITE 呼叫失败")]
    InviteFailed,
    /// 传输层绑定失败
    #[err(-5, "传输层绑定失败")]
    TransportBindFailed,
}
