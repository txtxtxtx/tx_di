//! CAN 总线插件错误码

use tx_error::CodeMsg;

/// CAN 总线插件业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("CAN")]

pub enum CanErr {
    /// 配置加载失败
    #[err(-1, "配置加载失败")]
    ConfigLoadFailed,
    /// 适配器打开失败
    #[err(-2, "适配器打开失败")]
    AdapterOpenFailed,
    /// 适配器初始化失败
    #[err(-3, "适配器初始化失败")]
    AdapterInitFailed,
    /// 适配器绑定失败
    #[err(-4, "适配器绑定失败")]
    AdapterBindFailed,
    /// 适配器未打开
    #[err(-5, "适配器未打开")]
    AdapterNotOpen,
    /// 适配器不可用
    #[err(-6, "适配器不可用")]
    AdapterUnavailable,
    /// 平台不支持
    #[err(-7, "平台不支持")]
    UnsupportedPlatform,
    /// 帧发送失败
    #[err(-8, "帧发送失败")]
    SendFailed,
    /// 无效的通道名称
    #[err(-9, "无效的通道名称")]
    InvalidChannel,
    /// DLL 未找到
    #[err(-10, "DLL 未找到")]
    DllNotFound,
    /// DLL 加载失败
    #[err(-11, "DLL 加载失败")]
    DllLoadFailed,
}
