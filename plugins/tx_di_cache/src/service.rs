//! CacheService trait — 缓存操作抽象
//!
//! 覆盖 Redis 原生 5 种数据类型（String/Hash/List/Set/SortedSet）及批量操作。

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;

use tx_error::AppResult;

/// 缓存服务抽象 trait
///
/// `CacheService` 是缓存操作的核心抽象，MemoryCache 和 RedisCache 都实现此 trait。
/// 通过 DI 注入 `Arc<dyn CacheService>` 使用，不感知底层实现。
#[async_trait]
pub trait CacheService: Send + Sync + 'static {
    // ── String (KV) 操作 ─────────────────────────────────────────────────

    /// 获取 key 对应的值
    async fn get(&self, key: &str) -> AppResult<Option<Vec<u8>>>;

    /// 设置 key 的值
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> AppResult<()>;

    /// 设置字符串值（便捷方法）
    async fn set_string(&self, key: &str, value: &str, ttl: Option<Duration>) -> AppResult<()> {
        self.set(key, value.as_bytes(), ttl).await
    }

    /// 删除 key
    async fn del(&self, key: &str) -> AppResult<()>;

    /// 检查 key 是否存在
    async fn exists(&self, key: &str) -> AppResult<bool>;

    /// 获取 key 的剩余存活时间（None 表示永不过期或 key 不存在）
    async fn ttl(&self, key: &str) -> AppResult<Option<Duration>>;

    /// 为 key 设置超时时间
    async fn expire(&self, key: &str, ttl: Duration) -> AppResult<()>;

    // ── Hash 操作 ─────────────────────────────────────────────────────────

    /// 获取 hash 中 field 的值
    async fn hget(&self, key: &str, field: &str) -> AppResult<Option<Vec<u8>>>;

    /// 设置 hash 中 field 的值
    async fn hset(&self, key: &str, field: &str, value: &[u8]) -> AppResult<()>;

    /// 删除 hash 中的一个或多个字段
    async fn hdel(&self, key: &str, field: &str) -> AppResult<()>;

    /// 获取 hash 的全部 field-value
    async fn hgetall(&self, key: &str) -> AppResult<HashMap<String, Vec<u8>>>;

    /// 获取 hash 的所有字段名
    async fn hkeys(&self, key: &str) -> AppResult<Vec<String>>;

    /// 获取 hash 的字段数量
    async fn hlen(&self, key: &str) -> AppResult<usize>;

    // ── List 操作 ─────────────────────────────────────────────────────────

    /// 从左侧插入一个或多个值
    async fn lpush(&self, key: &str, values: &[&[u8]]) -> AppResult<usize>;

    /// 从右侧插入一个或多个值
    async fn rpush(&self, key: &str, values: &[&[u8]]) -> AppResult<usize>;

    /// 从左侧弹出一个值
    async fn lpop(&self, key: &str) -> AppResult<Option<Vec<u8>>>;

    /// 从右侧弹出一个值
    async fn rpop(&self, key: &str) -> AppResult<Option<Vec<u8>>>;

    /// 获取列表指定范围的元素
    async fn lrange(&self, key: &str, start: i64, stop: i64) -> AppResult<Vec<Vec<u8>>>;

    /// 获取列表长度
    async fn llen(&self, key: &str) -> AppResult<usize>;

    // ── Set 操作 ──────────────────────────────────────────────────────────

    /// 向集合添加一个或多个成员
    async fn sadd(&self, key: &str, members: &[&[u8]]) -> AppResult<usize>;

    /// 从集合移除一个或多个成员
    async fn srem(&self, key: &str, members: &[&[u8]]) -> AppResult<usize>;

    /// 获取集合所有成员
    async fn smembers(&self, key: &str) -> AppResult<Vec<Vec<u8>>>;

    /// 判断 member 是否是集合的成员
    async fn sismember(&self, key: &str, member: &[u8]) -> AppResult<bool>;

    /// 获取集合成员数
    async fn scard(&self, key: &str) -> AppResult<usize>;

    // ── Sorted Set 操作 ───────────────────────────────────────────────────

    /// 向有序集合添加成员（member, score）
    async fn zadd(&self, key: &str, members: &[(&[u8], f64)]) -> AppResult<usize>;

    /// 从有序集合移除成员
    async fn zrem(&self, key: &str, members: &[&[u8]]) -> AppResult<usize>;

    /// 按排名范围获取成员（with_scores=true 时交替返回 member, score 的 bytes 表示）
    async fn zrange(&self, key: &str, start: i64, stop: i64, with_scores: bool) -> AppResult<Vec<Vec<u8>>>;

    /// 按 score 范围获取成员
    async fn zrangebyscore(&self, key: &str, min: f64, max: f64) -> AppResult<Vec<Vec<u8>>>;

    /// 获取成员的 score
    async fn zscore(&self, key: &str, member: &[u8]) -> AppResult<Option<f64>>;

    /// 获取有序集合成员数
    async fn zcard(&self, key: &str) -> AppResult<usize>;

    // ── 批量操作 ────────────────────────────────────────────────────────

    /// 批量获取多个 key
    async fn get_many(&self, keys: &[&str]) -> AppResult<Vec<Option<Vec<u8>>>>;

    /// 批量设置多个 key-value
    async fn set_many(&self, pairs: &[(&str, &[u8])], ttl: Option<Duration>) -> AppResult<()>;

    /// 批量删除多个 key
    async fn del_many(&self, keys: &[&str]) -> AppResult<usize>;

    // ── 清理操作 ────────────────────────────────────────────────────────

    /// 按模式匹配 key（支持 glob 风格，如 `user:*`）
    async fn keys(&self, pattern: &str) -> AppResult<Vec<String>>;

    /// 清空所有缓存
    async fn clear(&self) -> AppResult<()>;

    /// 清空指定前缀的所有缓存
    async fn clear_prefix(&self, prefix: &str) -> AppResult<usize>;
}
