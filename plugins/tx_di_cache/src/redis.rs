//! RedisCache — Redis 缓存实现（feature-gated）
//!
//! 仅当 feature="redis" 时编译。连接管理器使用 OnceLock 延迟初始化
//!（首次缓存操作时建立连接），避免在 Component 构建阶段执行异步操作。

#![cfg(feature = "redis")]

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tx_di_core::Component;
use tx_error::{AppError, AppResult};

use crate::config::CacheConfig;
use crate::err::CacheErr;
use crate::service::CacheService;

/// Redis 缓存组件
///
/// 连接管理器延迟初始化（首次操作时建立连接），
/// 仅在 Cargo feature `redis` 启用时可用。
#[derive(Component)]
#[component(as_trait = dyn CacheService)]
pub struct RedisCache {
    /// 缓存配置（自动注入）
    pub config: Arc<CacheConfig>,
    /// 延迟初始化的 Redis 连接管理器
    #[tx_cst(Arc::new(OnceLock::new()))]
    mgr: Arc<OnceLock<ConnectionManager>>,
}

impl RedisCache {
    /// 获取或初始化 Redis 连接管理器
    async fn get_mgr(&self) -> AppResult<&ConnectionManager> {
        self.mgr
            .get_or_try_init(|| async {
                let url = self
                    .config
                    .redis_url
                    .as_deref()
                    .unwrap_or("redis://127.0.0.1:6379");
                let client = redis::Client::open(url)
                    .map_err(|_| AppError::from_code(CacheErr::RedisConnectionFailed))?;
                ConnectionManager::new(client)
                    .await
                    .map_err(|_| AppError::from_code(CacheErr::RedisConnectionFailed))
            })
            .await
            .map_err(|e: AppError| e)
    }

    /// 构建真实 key（添加前缀）
    fn prefixed(&self, key: &str) -> String {
        if self.config.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.config.key_prefix, key)
        }
    }

    /// 将 RedisError 转为 AppError
    fn redis_err(e: redis::RedisError) -> AppError {
        AppError::with_context(CacheErr::RedisCommandError, e.to_string())
    }
}

#[async_trait]
impl CacheService for RedisCache {
    // ── String (KV) 操作 ─────────────────────────────────────────────────

    async fn get(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.get(&pk).await.map_err(Self::redis_err)
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> AppResult<()> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        if let Some(ttl) = ttl {
            conn.set_ex(&pk, value, ttl.as_secs() as usize).await.map_err(Self::redis_err)?;
        } else {
            conn.set(&pk, value).await.map_err(Self::redis_err)?;
        }
        Ok(())
    }

    async fn del(&self, key: &str) -> AppResult<()> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.del(&pk).await.map_err(Self::redis_err)?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.exists(&pk).await.map_err(Self::redis_err)
    }

    async fn ttl(&self, key: &str) -> AppResult<Option<Duration>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let secs: i64 = conn.ttl(&pk).await.map_err(Self::redis_err)?;
        Ok(if secs < 0 { None } else { Some(Duration::from_secs(secs as u64)) })
    }

    async fn expire(&self, key: &str, ttl: Duration) -> AppResult<()> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.expire(&pk, ttl.as_secs() as usize).await.map_err(Self::redis_err)?;
        Ok(())
    }

    // ── Hash 操作 ─────────────────────────────────────────────────────────

    async fn hget(&self, key: &str, field: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.hget(&pk, field).await.map_err(Self::redis_err)
    }

    async fn hset(&self, key: &str, field: &str, value: &[u8]) -> AppResult<()> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.hset(&pk, field, value).await.map_err(Self::redis_err)?;
        Ok(())
    }

    async fn hdel(&self, key: &str, field: &str) -> AppResult<()> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.hdel(&pk, field).await.map_err(Self::redis_err)?;
        Ok(())
    }

    async fn hgetall(&self, key: &str) -> AppResult<HashMap<String, Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.hgetall(&pk).await.map_err(Self::redis_err)
    }

    async fn hkeys(&self, key: &str) -> AppResult<Vec<String>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.hkeys(&pk).await.map_err(Self::redis_err)
    }

    async fn hlen(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.hlen(&pk).await.map_err(Self::redis_err)
    }

    // ── List 操作 ─────────────────────────────────────────────────────────

    async fn lpush(&self, key: &str, values: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let mut count = 0usize;
        for v in values.iter().rev() {
            count = conn.lpush(&pk, v).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }

    async fn rpush(&self, key: &str, values: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let mut count = 0usize;
        for v in values {
            count = conn.rpush(&pk, v).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }

    async fn lpop(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.lpop(&pk, None).await.map_err(Self::redis_err)
    }

    async fn rpop(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.rpop(&pk, None).await.map_err(Self::redis_err)
    }

    async fn lrange(&self, key: &str, start: i64, stop: i64) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.lrange(&pk, start, stop).await.map_err(Self::redis_err)
    }

    async fn llen(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.llen(&pk).await.map_err(Self::redis_err)
    }

    // ── Set 操作 ──────────────────────────────────────────────────────────

    async fn sadd(&self, key: &str, members: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let mut count = 0usize;
        for m in members {
            count = conn.sadd(&pk, m).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }

    async fn srem(&self, key: &str, members: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let mut count = 0usize;
        for m in members {
            count = conn.srem(&pk, m).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }

    async fn smembers(&self, key: &str) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.smembers(&pk).await.map_err(Self::redis_err)
    }

    async fn sismember(&self, key: &str, member: &[u8]) -> AppResult<bool> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.sismember(&pk, member).await.map_err(Self::redis_err)
    }

    async fn scard(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.scard(&pk).await.map_err(Self::redis_err)
    }

    // ── Sorted Set 操作 ───────────────────────────────────────────────────

    async fn zadd(&self, key: &str, members: &[(&[u8], f64)]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let mut count = 0usize;
        for (member, score) in members {
            count = conn.zadd(&pk, member, *score).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }

    async fn zrem(&self, key: &str, members: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        let mut count = 0usize;
        for m in members {
            count = conn.zrem(&pk, m).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }

    async fn zrange(&self, key: &str, start: i64, stop: i64, with_scores: bool) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        if with_scores {
            let val: Vec<String> = conn.zrange_withscores(&pk, start, stop).await.map_err(Self::redis_err)?;
            Ok(val.into_iter().map(|s| s.into_bytes()).collect())
        } else {
            conn.zrange(&pk, start, stop).await.map_err(Self::redis_err)
        }
    }

    async fn zrangebyscore(&self, key: &str, min: f64, max: f64) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.zrangebyscore(&pk, min, max).await.map_err(Self::redis_err)
    }

    async fn zscore(&self, key: &str, member: &[u8]) -> AppResult<Option<f64>> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.zscore(&pk, member).await.map_err(Self::redis_err)
    }

    async fn zcard(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut conn = self.get_mgr().await?.clone();
        conn.zcard(&pk).await.map_err(Self::redis_err)
    }

    // ── 批量操作 ────────────────────────────────────────────────────────

    async fn get_many(&self, keys: &[&str]) -> AppResult<Vec<Option<Vec<u8>>>> {
        let pks: Vec<String> = keys.iter().map(|k| self.prefixed(k)).collect();
        let refs: Vec<&str> = pks.iter().map(|s| s.as_str()).collect();
        let mut conn = self.get_mgr().await?.clone();
        conn.mget(&refs).await.map_err(Self::redis_err)
    }

    async fn set_many(&self, pairs: &[(&str, &[u8])], ttl: Option<Duration>) -> AppResult<()> {
        let mut conn = self.get_mgr().await?.clone();
        for (key, value) in pairs {
            let pk = self.prefixed(key);
            if let Some(ttl) = ttl {
                conn.set_ex(&pk, value, ttl.as_secs() as usize).await.map_err(Self::redis_err)?;
            } else {
                conn.set(&pk, value).await.map_err(Self::redis_err)?;
            }
        }
        Ok(())
    }

    async fn del_many(&self, keys: &[&str]) -> AppResult<usize> {
        let pks: Vec<String> = keys.iter().map(|k| self.prefixed(k)).collect();
        let refs: Vec<&str> = pks.iter().map(|s| s.as_str()).collect();
        let mut conn = self.get_mgr().await?.clone();
        conn.del(&refs).await.map_err(Self::redis_err)
    }

    // ── 清理操作 ────────────────────────────────────────────────────────

    async fn keys(&self, pattern: &str) -> AppResult<Vec<String>> {
        let pk = self.prefixed(pattern);
        let mut conn = self.get_mgr().await?.clone();
        let val: Vec<String> = conn.keys(&pk).await.map_err(Self::redis_err)?;
        if self.config.key_prefix.is_empty() {
            Ok(val)
        } else {
            Ok(val.into_iter()
                .filter_map(|k| k.strip_prefix(&self.config.key_prefix).map(String::from))
                .collect())
        }
    }

    async fn clear(&self) -> AppResult<()> {
        let mut conn = self.get_mgr().await?.clone();
        redis::cmd("FLUSHDB").query_async(&mut *conn).await.map_err(Self::redis_err)?;
        Ok(())
    }

    async fn clear_prefix(&self, prefix: &str) -> AppResult<usize> {
        let pk = self.prefixed(prefix);
        let pattern = format!("{}*", pk);
        let mut conn = self.get_mgr().await?.clone();
        let keys: Vec<String> = conn.keys(&pattern).await.map_err(Self::redis_err)?;
        let count = keys.len();
        if !keys.is_empty() {
            let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
            conn.del(&refs).await.map_err(Self::redis_err)?;
        }
        Ok(count)
    }
}
