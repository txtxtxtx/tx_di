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
    /// BYE 请求失败
    #[err(-6, "BYE 请求失败")]
    ByeFailed,
    /// CANCEL 请求失败
    #[err(-7, "CANCEL 请求失败")]
    CancelFailed,
    /// MESSAGE 请求失败
    #[err(-8, "MESSAGE 请求失败")]
    MessageFailed,
    /// SIP 端点已设置（重复初始化）
    #[err(-9, "SIP 端点已设置")]
    EndpointAlreadySet,
    /// 取消令牌已存在
    #[err(-10, "取消令牌已存在")]
    TokenAlreadySet,
    /// SIP 回复失败
    #[err(-11, "SIP 回复失败")]
    ReplyFailed,
    /// SIP 事务已被取出（无法再回复）
    #[err(-12, "SIP 事务已被取出")]
    TransactionMissing,
}
