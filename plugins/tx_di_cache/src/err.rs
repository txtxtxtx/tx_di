//! 缓存插件错误码

use tx_error::CodeMsg;

/// 缓存插件业务错误码。
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("CACHE")]
pub enum CacheErr {
    /// 键类型不匹配（如用 Hash 操作访问 String 类型）
    #[err(1001, "类型错误")]
    TypeError,
    /// 缓存已满（内存限制）
    #[err(1002, "缓存已满")]
    ResourceExhausted,
    /// Redis 连接失败
    #[err(2001, "Redis 连接失败")]
    RedisConnectionFailed,
    /// Redis 命令执行错误
    #[err(2002, "Redis 命令错误")]
    RedisCommandError,
}
