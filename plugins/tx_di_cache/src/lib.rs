//! tx_di_cache — 缓存插件
//!
//! 提供统一的 `CacheService` trait 抽象层，支持多数据类型（String/Hash/List/Set/SortedSet），
//! 默认使用内存缓存（MemoryCache），可选 Redis 后端（feature = "redis"）。
//!
//! # 快速开始
//!
//! ```toml
//! # Cargo.toml
//! tx_di_cache = { path = "plugins/tx_di_cache" }
//!
//! # 启用 Redis：
//! tx_di_cache = { path = "plugins/tx_di_cache", features = ["redis"] }
//! ```
//!
//! ```toml
//! # configs/cache_config.toml
//! [cache_config]
//! default_ttl_secs = 3600
//! key_prefix = ""
//! memory_max_capacity = 10000
//! redis_url = "redis://127.0.0.1:6379"
//! redis_pool_size = 10
//! ```
//!
//! # 业务代码中使用
//!
//! ```ignore
//! use tx_di_cache::CacheService;
//!
//! let cache: Arc<dyn CacheService> = inject_trait_from_store::<dyn CacheService>(store);
//! cache.set("key", b"value", Some(Duration::from_secs(60))).await?;
//! let val = cache.get("key").await?;
//! ```

mod config;
mod err;
mod memory;
mod service;

#[cfg(feature = "redis")]
mod redis;

pub use config::CacheConfig;
pub use service::CacheService;
pub use memory::MemoryCache;

#[cfg(feature = "redis")]
pub use redis::RedisCache;
