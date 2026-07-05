//! 缓存配置组件

use serde::Deserialize;
use tx_di_core::{Component, RIE, Store};

/// 缓存配置
///
/// 从 TOML 配置文件 `[cache_config]` 节自动加载。
///
/// ```toml
/// [cache_config]
/// default_ttl_secs = 3600
/// key_prefix = "myapp:"
/// memory_max_capacity = 10000
/// redis_url = "redis://127.0.0.1:6379"
/// redis_pool_size = 10
/// ```
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf, init, init_sort = i32::MIN + 1)]
pub struct CacheConfig {
    /// 默认 TTL（秒），不设或 0 表示永不过期
    #[serde(default = "default_ttl")]
    pub default_ttl_secs: u64,

    /// 键前缀（用于隔离不同应用的缓存键）
    #[serde(default)]
    pub key_prefix: String,

    /// 内存缓存最大条目数（0 = 无限制）
    #[serde(default = "default_max_capacity")]
    pub memory_max_capacity: usize,

    /// Redis 连接 URL（仅 feature="redis" 时有效）
    ///
    /// 示例：`redis://127.0.0.1:6379`、`rediss://user:pass@host:6380/0`
    #[serde(default)]
    pub redis_url: Option<String>,

    /// Redis 连接池大小（仅 feature="redis" 时有效）
    #[serde(default = "default_pool_size")]
    pub redis_pool_size: u32,

    /// Redis 连接超时（秒）
    #[serde(default = "default_timeout")]
    pub redis_timeout_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl_secs: default_ttl(),
            key_prefix: String::new(),
            memory_max_capacity: default_max_capacity(),
            redis_url: None,
            redis_pool_size: default_pool_size(),
            redis_timeout_secs: default_timeout(),
        }
    }
}

fn init(this: &mut CacheConfig, _store: &Store) -> RIE<()> {
    tracing::debug!(
        default_ttl_ms = this.default_ttl_secs,
        key_prefix = %this.key_prefix,
        memory_max_capacity = this.memory_max_capacity,
        redis_pool_size = this.redis_pool_size,
        "缓存配置已加载"
    );
    Ok(())
}

fn default_ttl() -> u64 { 3600 }
fn default_max_capacity() -> usize { 10000 }
fn default_pool_size() -> u32 { 10 }
fn default_timeout() -> u64 { 5 }
