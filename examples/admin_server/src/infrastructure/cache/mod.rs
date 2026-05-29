//! 缓存层
//!
//! 提供简单的内存缓存实现，用于缓存用户权限等热点数据。

use std::collections::HashMap;
use std::sync::RwLock;
use tx_di_core::tx_comp;

/// 简单内存缓存
///
/// 存储键值对，带过期时间。
/// 可用于缓存用户信息、权限数据等。
#[derive(Debug, Default)]
#[tx_comp]
pub struct MemoryCache {
    store: RwLock<HashMap<String, CacheEntry>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<i64>,
}

impl MemoryCache {
    /// 设置缓存
    pub fn set(&self, key: &str, value: serde_json::Value, ttl_secs: Option<u64>) {
        let expires_at = ttl_secs.map(|ttl| chrono::Utc::now().timestamp() + ttl as i64);
        self.store.write().unwrap().insert(
            key.to_string(),
            CacheEntry { value, expires_at },
        );
    }

    /// 获取缓存（自动处理过期）
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let store = self.store.read().unwrap();
        let entry = store.get(key)?;

        // 检查过期
        if let Some(expires) = entry.expires_at {
            if chrono::Utc::now().timestamp() > expires {
                drop(store);
                self.remove(key);
                return None;
            }
        }

        Some(entry.value.clone())
    }

    /// 删除缓存
    pub fn remove(&self, key: &str) {
        self.store.write().unwrap().remove(key);
    }

    /// 清空缓存
    pub fn clear(&self) {
        self.store.write().unwrap().clear();
    }
}
