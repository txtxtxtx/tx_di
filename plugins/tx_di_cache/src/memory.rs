//! MemoryCache — 内存缓存实现
//!
//! 使用 `DashMap` 实现并发安全的进程内缓存，支持全部 5 种数据类型。
//! TTL 采用惰性过期（读取时检查），可选后台定时清理任务。

use std::collections::{HashMap, HashSet, VecDeque, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;
use tx_di_core::{Component, DepsTuple};
use tx_error::{AppError, AppResult};

use crate::config::CacheConfig;
use crate::err::CacheErr;
use crate::service::CacheService;

// ═══════════════════════════════════════════════════════════════════════════════
// 内部数据结构
// ═══════════════════════════════════════════════════════════════════════════════

/// 缓存值的内部表示（区分数据类型）
enum CacheValue {
    StringValue {
        data: Vec<u8>,
        expires_at: Option<Instant>,
    },
    HashValue {
        fields: HashMap<String, Vec<u8>>,
        expires_at: Option<Instant>,
    },
    ListValue {
        deque: VecDeque<Vec<u8>>,
        expires_at: Option<Instant>,
    },
    SetValue {
        members: HashSet<Vec<u8>>,
        expires_at: Option<Instant>,
    },
    SortedSetValue {
        members: BTreeSet<ZMember>,
        expires_at: Option<Instant>,
    },
}

/// 有序集合成员
#[derive(Debug, Clone, PartialEq)]
struct ZMember {
    member: Vec<u8>,
    score: f64,
}

impl Eq for ZMember {}

impl Ord for ZMember {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.member.cmp(&other.member))
    }
}

impl PartialOrd for ZMember {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MemoryCache Component
// ═══════════════════════════════════════════════════════════════════════════════

/// 内存缓存组件（默认实现）
///
/// 进程内缓存，使用 DashMap 实现并发安全的 KV 存储，支持所有 5 种数据类型。
/// 当启用 Redis feature 时，可通过配置切换为 RedisCache。
///
/// # Deps
///
/// - `Arc<CacheConfig>` — 缓存配置
#[derive(Component)]
#[component(as_trait = dyn CacheService)]
pub struct MemoryCache {
    /// 缓存配置（自动注入）
    pub config: Arc<CacheConfig>,
    /// 并发缓存存储
    #[tx_cst(Arc::new(DashMap::<String, CacheValue>::new()))]
    store: Arc<DashMap<String, CacheValue>>,
}

impl MemoryCache {
    /// 创建 MemoryCache 实例（用于非 DI 场景的测试）
    pub fn new(config: Arc<CacheConfig>) -> Self {
        Self {
            config,
            store: Arc::new(DashMap::new()),
        }
    }

    /// 构建真实 key（添加前缀）
    fn prefixed(&self, key: &str) -> String {
        if self.config.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.config.key_prefix, key)
        }
    }

    /// 检查并惰性删除过期条目
    fn check_expiry(&self, key: &str) -> bool {
        if let Some(entry) = self.store.get(key) {
            let expired = match entry.value() {
                CacheValue::StringValue { expires_at, .. } => {
                    expires_at.map_or(false, |e| Instant::now() >= e)
                }
                CacheValue::HashValue { expires_at, .. } => {
                    expires_at.map_or(false, |e| Instant::now() >= e)
                }
                CacheValue::ListValue { expires_at, .. } => {
                    expires_at.map_or(false, |e| Instant::now() >= e)
                }
                CacheValue::SetValue { expires_at, .. } => {
                    expires_at.map_or(false, |e| Instant::now() >= e)
                }
                CacheValue::SortedSetValue { expires_at, .. } => {
                    expires_at.map_or(false, |e| Instant::now() >= e)
                }
            };
            if expired {
                drop(entry);
                self.store.remove(key);
                return true;
            }
        }
        false
    }

    /// 计算过期时间点
    fn expires_at(ttl: Option<Duration>) -> Option<Instant> {
        ttl.map(|d| Instant::now() + d)
    }

    /// 检查缓存是否已满（达到 max_capacity）
    fn is_full(&self) -> bool {
        let max = self.config.memory_max_capacity;
        max > 0 && self.store.len() >= max
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CacheService 实现
// ═══════════════════════════════════════════════════════════════════════════════

#[async_trait]
impl CacheService for MemoryCache {
    // ── String (KV) 操作 ─────────────────────────────────────────────────

    async fn get(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::StringValue { data, .. } => Ok(Some(data.clone())),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> AppResult<()> {
        let pk = self.prefixed(key);
        if self.is_full() && !self.store.contains_key(&pk) {
            return Err(AppError::from_code(CacheErr::ResourceExhausted));
        }
        self.store.insert(pk, CacheValue::StringValue {
            data: value.to_vec(),
            expires_at: Self::expires_at(ttl),
        });
        Ok(())
    }

    async fn del(&self, key: &str) -> AppResult<()> {
        let pk = self.prefixed(key);
        self.store.remove(&pk);
        Ok(())
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        Ok(self.store.contains_key(&pk))
    }

    async fn ttl(&self, key: &str) -> AppResult<Option<Duration>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => {
                let expires_at = match entry.value() {
                    CacheValue::StringValue { expires_at, .. } => expires_at,
                    CacheValue::HashValue { expires_at, .. } => expires_at,
                    CacheValue::ListValue { expires_at, .. } => expires_at,
                    CacheValue::SetValue { expires_at, .. } => expires_at,
                    CacheValue::SortedSetValue { expires_at, .. } => expires_at,
                };
                match expires_at {
                    Some(exp) => {
                        let remaining = exp.saturating_duration_since(Instant::now());
                        Ok(Some(remaining))
                    }
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn expire(&self, key: &str, ttl: Duration) -> AppResult<()> {
        let pk = self.prefixed(key);
        if let Some(mut entry) = self.store.get_mut(&pk) {
            let expires_at = Some(Instant::now() + ttl);
            match entry.value_mut() {
                CacheValue::StringValue { expires_at: e, .. } => *e = expires_at,
                CacheValue::HashValue { expires_at: e, .. } => *e = expires_at,
                CacheValue::ListValue { expires_at: e, .. } => *e = expires_at,
                CacheValue::SetValue { expires_at: e, .. } => *e = expires_at,
                CacheValue::SortedSetValue { expires_at: e, .. } => *e = expires_at,
            }
        }
        Ok(())
    }

    // ── Hash 操作 ─────────────────────────────────────────────────────────

    async fn hget(&self, key: &str, field: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::HashValue { fields, .. } => Ok(fields.get(field).cloned()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(None),
        }
    }

    async fn hset(&self, key: &str, field: &str, value: &[u8]) -> AppResult<()> {
        let pk = self.prefixed(key);
        if self.is_full() && !self.store.contains_key(&pk) {
            return Err(AppError::from_code(CacheErr::ResourceExhausted));
        }
        self.store
            .entry(pk)
            .and_modify(|v| {
                if let CacheValue::HashValue { fields, .. } = v {
                    fields.insert(field.to_string(), value.to_vec());
                }
            })
            .or_insert_with(|| CacheValue::HashValue {
                fields: HashMap::from_iter([(field.to_string(), value.to_vec())]),
                expires_at: None,
            });
        Ok(())
    }

    async fn hdel(&self, key: &str, field: &str) -> AppResult<()> {
        let pk = self.prefixed(key);
        if let Some(mut entry) = self.store.get_mut(&pk) {
            if let CacheValue::HashValue { fields, .. } = entry.value_mut() {
                fields.remove(field);
                if fields.is_empty() {
                    drop(entry);
                    self.store.remove(&pk);
                }
            }
        }
        Ok(())
    }

    async fn hgetall(&self, key: &str) -> AppResult<HashMap<String, Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::HashValue { fields, .. } => Ok(fields.clone()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(HashMap::new()),
        }
    }

    async fn hkeys(&self, key: &str) -> AppResult<Vec<String>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::HashValue { fields, .. } => Ok(fields.keys().cloned().collect()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(Vec::new()),
        }
    }

    async fn hlen(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::HashValue { fields, .. } => Ok(fields.len()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(0),
        }
    }

    // ── List 操作 ─────────────────────────────────────────────────────────

    async fn lpush(&self, key: &str, values: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        if self.is_full() && !self.store.contains_key(&pk) {
            return Err(AppError::from_code(CacheErr::ResourceExhausted));
        }
        let count = values.len();
        self.store
            .entry(pk)
            .and_modify(|v| {
                if let CacheValue::ListValue { deque, .. } = v {
                    for val in values.iter().rev() {
                        deque.push_front(val.to_vec());
                    }
                }
            })
            .or_insert_with(|| CacheValue::ListValue {
                deque: VecDeque::from_iter(values.iter().map(|v| v.to_vec())),
                expires_at: None,
            });
        Ok(count)
    }

    async fn rpush(&self, key: &str, values: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        if self.is_full() && !self.store.contains_key(&pk) {
            return Err(AppError::from_code(CacheErr::ResourceExhausted));
        }
        let count = values.len();
        self.store
            .entry(pk)
            .and_modify(|v| {
                if let CacheValue::ListValue { deque, .. } = v {
                    for val in values {
                        deque.push_back(val.to_vec());
                    }
                }
            })
            .or_insert_with(|| CacheValue::ListValue {
                deque: VecDeque::from_iter(values.iter().map(|v| v.to_vec())),
                expires_at: None,
            });
        Ok(count)
    }

    async fn lpop(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        match self.store.get_mut(&pk) {
            Some(mut entry) => match entry.value_mut() {
                CacheValue::ListValue { deque, .. } => {
                    let val = deque.pop_front();
                    if deque.is_empty() {
                        drop(entry);
                        self.store.remove(&pk);
                    }
                    Ok(val)
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(None),
        }
    }

    async fn rpop(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let pk = self.prefixed(key);
        match self.store.get_mut(&pk) {
            Some(mut entry) => match entry.value_mut() {
                CacheValue::ListValue { deque, .. } => {
                    let val = deque.pop_back();
                    if deque.is_empty() {
                        drop(entry);
                        self.store.remove(&pk);
                    }
                    Ok(val)
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(None),
        }
    }

    async fn lrange(&self, key: &str, start: i64, stop: i64) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::ListValue { deque, .. } => {
                    let len = deque.len() as i64;
                    let s = if start < 0 { (len + start).max(0) } else { start.min(len - 1) };
                    let e = if stop < 0 { len + stop } else { stop.min(len - 1) };
                    let result: Vec<Vec<u8>> = deque
                        .iter()
                        .skip(s as usize)
                        .take((e - s + 1).max(0) as usize)
                        .cloned()
                        .collect();
                    Ok(result)
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(Vec::new()),
        }
    }

    async fn llen(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::ListValue { deque, .. } => Ok(deque.len()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(0),
        }
    }

    // ── Set 操作 ──────────────────────────────────────────────────────────

    async fn sadd(&self, key: &str, members: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        if self.is_full() && !self.store.contains_key(&pk) {
            return Err(AppError::from_code(CacheErr::ResourceExhausted));
        }
        let mut added = 0;
        self.store
            .entry(pk)
            .and_modify(|v| {
                if let CacheValue::SetValue { members: m, .. } = v {
                    for val in members {
                        if m.insert(val.to_vec()) {
                            added += 1;
                        }
                    }
                }
            })
            .or_insert_with(|| {
                added = members.len();
                CacheValue::SetValue {
                    members: HashSet::from_iter(members.iter().map(|v| v.to_vec())),
                    expires_at: None,
                }
            });
        Ok(added)
    }

    async fn srem(&self, key: &str, members: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut removed = 0;
        if let Some(mut entry) = self.store.get_mut(&pk) {
            if let CacheValue::SetValue { members: m, .. } = entry.value_mut() {
                for val in members {
                    if m.remove(&val.to_vec()) {
                        removed += 1;
                    }
                }
                if m.is_empty() {
                    drop(entry);
                    self.store.remove(&pk);
                }
            }
        }
        Ok(removed)
    }

    async fn smembers(&self, key: &str) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SetValue { members, .. } => {
                    Ok(members.iter().cloned().collect())
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(Vec::new()),
        }
    }

    async fn sismember(&self, key: &str, member: &[u8]) -> AppResult<bool> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SetValue { members, .. } => Ok(members.contains(&member.to_vec())),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(false),
        }
    }

    async fn scard(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SetValue { members, .. } => Ok(members.len()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(0),
        }
    }

    // ── Sorted Set 操作 ───────────────────────────────────────────────────

    async fn zadd(&self, key: &str, members: &[(&[u8], f64)]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        if self.is_full() && !self.store.contains_key(&pk) {
            return Err(AppError::from_code(CacheErr::ResourceExhausted));
        }
        let mut added = 0;
        self.store
            .entry(pk)
            .and_modify(|v| {
                if let CacheValue::SortedSetValue { members: m, .. } = v {
                    for (member, score) in members {
                        if m.insert(ZMember {
                            member: member.to_vec(),
                            score: *score,
                        }) {
                            added += 1;
                        }
                    }
                }
            })
            .or_insert_with(|| {
                added = members.len();
                CacheValue::SortedSetValue {
                    members: BTreeSet::from_iter(members.iter().map(|(m, s)| ZMember {
                        member: m.to_vec(),
                        score: *s,
                    })),
                    expires_at: None,
                }
            });
        Ok(added)
    }

    async fn zrem(&self, key: &str, members: &[&[u8]]) -> AppResult<usize> {
        let pk = self.prefixed(key);
        let mut removed = 0;
        if let Some(mut entry) = self.store.get_mut(&pk) {
            if let CacheValue::SortedSetValue { members: m, .. } = entry.value_mut() {
                for member in members {
                    let to_remove: Vec<ZMember> = m
                        .iter()
                        .filter(|zm| zm.member == member.to_vec())
                        .cloned()
                        .collect();
                    for zm in to_remove {
                        m.remove(&zm);
                        removed += 1;
                    }
                }
                if m.is_empty() {
                    drop(entry);
                    self.store.remove(&pk);
                }
            }
        }
        Ok(removed)
    }

    async fn zrange(&self, key: &str, start: i64, stop: i64, with_scores: bool) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SortedSetValue { members, .. } => {
                    let vec: Vec<&ZMember> = members.iter().collect();
                    let len = vec.len() as i64;
                    let s = if start < 0 { (len + start).max(0) } else { start.min(len - 1).max(0) };
                    let e = if stop < 0 { len + stop } else { stop.min(len - 1) };
                    let mut result = Vec::new();
                    for i in s as usize..=(e as usize).min(vec.len().saturating_sub(1)) {
                        result.push(vec[i].member.clone());
                        if with_scores {
                            result.push(vec[i].score.to_le_bytes().to_vec());
                        }
                    }
                    Ok(result)
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(Vec::new()),
        }
    }

    async fn zrangebyscore(&self, key: &str, min: f64, max: f64) -> AppResult<Vec<Vec<u8>>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SortedSetValue { members, .. } => {
                    let result: Vec<Vec<u8>> = members
                        .iter()
                        .filter(|zm| zm.score >= min && zm.score <= max)
                        .map(|zm| zm.member.clone())
                        .collect();
                    Ok(result)
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(Vec::new()),
        }
    }

    async fn zscore(&self, key: &str, member: &[u8]) -> AppResult<Option<f64>> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SortedSetValue { members, .. } => {
                    Ok(members
                        .iter()
                        .find(|zm| zm.member == member.to_vec())
                        .map(|zm| zm.score))
                }
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(None),
        }
    }

    async fn zcard(&self, key: &str) -> AppResult<usize> {
        let pk = self.prefixed(key);
        self.check_expiry(&pk);
        match self.store.get(&pk) {
            Some(entry) => match entry.value() {
                CacheValue::SortedSetValue { members, .. } => Ok(members.len()),
                _ => Err(AppError::from_code(CacheErr::TypeError)),
            },
            None => Ok(0),
        }
    }

    // ── 批量操作 ────────────────────────────────────────────────────────

    async fn get_many(&self, keys: &[&str]) -> AppResult<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    async fn set_many(&self, pairs: &[(&str, &[u8])], ttl: Option<Duration>) -> AppResult<()> {
        for (key, value) in pairs {
            self.set(key, value, ttl).await?;
        }
        Ok(())
    }

    async fn del_many(&self, keys: &[&str]) -> AppResult<usize> {
        let mut count = 0;
        for key in keys {
            let pk = self.prefixed(key);
            if self.store.remove(&pk).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    // ── 清理操作 ────────────────────────────────────────────────────────

    async fn keys(&self, pattern: &str) -> AppResult<Vec<String>> {
        let regex_pattern = pattern_to_regex(pattern);
        let mut results = Vec::new();
        for entry in self.store.iter() {
            let raw_key = entry.key();
            if let Some(stripped) = raw_key.strip_prefix(&self.config.key_prefix) {
                if regex_match(stripped, &regex_pattern) {
                    results.push(stripped.to_string());
                }
            } else if regex_match(raw_key, &regex_pattern) {
                results.push(raw_key.clone());
            }
        }
        Ok(results)
    }

    async fn clear(&self) -> AppResult<()> {
        self.store.clear();
        Ok(())
    }

    async fn clear_prefix(&self, prefix: &str) -> AppResult<usize> {
        let full_prefix = self.prefixed(prefix);
        let keys: Vec<String> = self.store
            .iter()
            .filter(|e| e.key().starts_with(&full_prefix))
            .map(|e| e.key().clone())
            .collect();
        let count = keys.len();
        for key in &keys {
            self.store.remove(key);
        }
        Ok(count)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 工具函数
// ═══════════════════════════════════════════════════════════════════════════════

/// 将简单的 glob pattern 转为正则表达式
fn pattern_to_regex(pattern: &str) -> String {
    let mut re = String::with_capacity(pattern.len() + 4);
    re.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '|' | '^' | '$' => {
                re.push('\\');
                re.push(ch);
            }
            _ => re.push(ch),
        }
    }
    re.push('$');
    re
}

/// 使用 glob-style 匹配（支持 `*` 和 `?`）
fn regex_match(input: &str, _pattern: &str) -> bool {
    // 实际使用的是 glob 风格，不是完整正则
    // pattern_to_regex 转换后得到的是简化正则，这里直接使用简单的贪心匹配
    simple_glob_match(input, _pattern)
}

/// 递归 glob 匹配（仅支持 * 和 ?）
fn simple_glob_match(input: &str, pattern: &str) -> bool {
    let input_chars: Vec<char> = input.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let (il, pl) = (input_chars.len(), pattern_chars.len());
    let (mut i, mut p) = (0, 0);
    let (mut star_i, mut star_p): (Option<usize>, Option<usize>) = (None, None);

    while i < il {
        if p < pl && (pattern_chars[p] == input_chars[i] || pattern_chars[p] == '?') {
            i += 1;
            p += 1;
        } else if p < pl && pattern_chars[p] == '*' {
            star_i = Some(i);
            star_p = Some(p + 1);
            p += 1;
        } else if let (Some(si), Some(sp)) = (star_i, star_p) {
            i = si + 1;
            p = sp;
            star_i = Some(i);
        } else {
            return false;
        }
    }

    while p < pl && pattern_chars[p] == '*' {
        p += 1;
    }

    p >= pl
}
