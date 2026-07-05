//! DynamicConfig — 动态配置容器
//!
//! 支持运行时原子更新配置 + watch channel 广播变更事件。
//! 业务组件可以订阅配置变更，实现热更新。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use tokio::sync::watch;

/// 配置变更事件
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// 配置键（data_id）
    pub key: String,
    /// 新版本号
    pub version: u64,
}

/// 动态配置容器 — 原子更新 + 版本号追踪 + 变更广播
///
/// # 示例
///
/// ```ignore
/// let dc = DynamicConfig::new(100u32, "rate_limit");
/// assert_eq!(*dc.get(), 100);
///
/// // 订阅变更
/// let mut rx = dc.subscribe();
/// tokio::spawn(async move {
///     while rx.changed().await.is_ok() {
///         println!("配置已变更");
///     }
/// });
///
/// // 更新配置（触发通知）
/// dc.update(200);
/// ```
pub struct DynamicConfig<T> {
    inner: Arc<RwLock<T>>,
    key: String,
    version: Arc<AtomicU64>,
    /// 变更事件发送端
    tx: watch::Sender<ConfigChangeEvent>,
}

impl<T: Clone + Send + Sync + 'static> DynamicConfig<T> {
    /// 创建动态配置容器
    pub fn new(initial: T, key: impl Into<String>) -> Self {
        let key = key.into();
        let (tx, _) = watch::channel(ConfigChangeEvent {
            key: key.clone(),
            version: 0,
        });
        Self {
            inner: Arc::new(RwLock::new(initial)),
            key,
            version: Arc::new(AtomicU64::new(0)),
            tx,
        }
    }

    /// 获取当前配置（快照克隆）
    pub fn get(&self) -> T {
        self.inner.read().unwrap().clone()
    }

    /// 获取内部 `RwLock` 引用（用于读取而不克隆）
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }

    /// 更新配置（原子操作，触发通知）
    pub fn update(&self, new_val: T) {
        let mut w = self.inner.write().unwrap();
        *w = new_val;
        let ver = self.version.fetch_add(1, Ordering::Release) + 1;
        let _ = self.tx.send(ConfigChangeEvent {
            key: self.key.clone(),
            version: ver,
        });
    }

    /// 获取当前版本号（用于判断是否变更）
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// 订阅变更事件
    pub fn subscribe(&self) -> watch::Receiver<ConfigChangeEvent> {
        self.tx.subscribe()
    }

    /// 配置键名
    pub fn key(&self) -> &str {
        &self.key
    }
}
