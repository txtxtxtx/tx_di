//! NacosClient — 配置中心 + 服务注册的单连接统一封装
//!
//! **非 DI 组件**：不注册进容器，由应用入口（`app_loop!` 宏或手动）在 App 之外创建一次，
//! 全程复用，与 App 生命周期解耦。

use std::sync::Arc;

use serde::de::DeserializeOwned;
use tokio::sync::watch;
use tx_di_core::RIE;
use tx_error::AppError;

use crate::config::RegistryConfig;
use crate::dynamic_config::DynamicConfig;
use crate::model::{ServiceEndpoint, ServiceInstance};
use crate::nacos::config_impl::NacosConfigCenter;
use crate::nacos::registry_impl::NacosServiceRegistry;
use crate::traits::{ConfigCenter, ServiceRegistry};

/// 单连接 Nacos 客户端：配置中心（ConfigService）+ 服务注册（NamingService）
///
/// 两个 SDK service 使用相同 `ClientProps` 构建，SDK 内部共享底层 gRPC 连接。
pub struct NacosClient {
    /// 配置中心客户端
    pub config: Arc<dyn ConfigCenter>,
    /// 服务注册/发现客户端
    pub naming: Arc<dyn ServiceRegistry>,
    /// 客户端配置
    pub cfg: RegistryConfig,
}

impl NacosClient {
    /// 构建单连接 Nacos 客户端（配置中心 + 服务注册）
    pub async fn connect(cfg: &RegistryConfig) -> RIE<Self> {
        let config = Arc::new(NacosConfigCenter::new(cfg).await?) as Arc<dyn ConfigCenter>;
        let naming = Arc::new(NacosServiceRegistry::new(cfg).await?) as Arc<dyn ServiceRegistry>;
        Ok(Self {
            config,
            naming,
            cfg: cfg.clone(),
        })
    }

    /// 按 `enabled` 开关连接：`enabled=false` 时返回 `None`（纯本地模式）
    pub async fn connect_if_enabled(cfg: &RegistryConfig) -> RIE<Option<Self>> {
        if !cfg.enabled {
            return Ok(None);
        }
        Ok(Some(Self::connect(cfg).await?))
    }

    // ── 配置中心能力 ────────────────────────────────────────────────────

    /// 拉取远程配置（不存在 → `None`）
    pub async fn pull_config(&self, data_id: &str) -> RIE<Option<String>> {
        self.config.get_config(data_id, &self.cfg.group).await
    }

    /// 合并配置：远程 TOML 深合并覆盖本地（`remote=None` 时原样返回 `local`）
    pub fn merge_config(&self, local: toml::Value, remote: Option<String>) -> RIE<toml::Value> {
        let Some(remote) = remote else {
            return Ok(local);
        };
        let remote_val: toml::Value = toml::from_str(&remote)
            .map_err(|e| AppError::from(format!("远程配置 TOML 解析失败: {e}")))?;
        Ok(merge_toml(local, remote_val))
    }

    /// 监听主配置变更：注册一次 listener，变更时递增版本号
    ///
    /// 返回 `watch::Receiver`，供 `app_loop!` 内 `select!` 使用。
    /// 返回后立刻有初始值 `0`，变更后为 `1,2,...`。
    pub fn watch_config(&self, data_id: &str) -> watch::Receiver<u64> {
        let (tx, rx) = watch::channel(0u64);
        let cc = self.config.clone();
        let did = data_id.to_string();
        let grp = self.cfg.group.clone();
        tokio::spawn(async move {
            cc.listen_config(
                &did,
                &grp,
                Box::new(move |_| {
                    let _ = tx.send_modify(|v| *v += 1);
                }),
            )
            .await;
        });
        rx
    }

    /// 获取**纯查询型业务参数**的动态配置（热更新，不触发应用重启）
    ///
    /// # 示例
    /// ```rust,ignore
    /// let dc = client.dynamic_config::<RateLimitConf>("rate_limit").await?;
    /// let current = dc.get();          // 当前值
    /// let mut rx = dc.subscribe();     // 热更新订阅
    /// ```
    pub async fn dynamic_config<T>(&self, data_id: &str) -> RIE<DynamicConfig<T>>
    where
        T: DeserializeOwned + Clone + Send + Sync + Default + 'static,
    {
        let cc = self.config.clone();
        let initial = match cc.get_config(data_id, &self.cfg.group).await? {
            Some(raw) => serde_json::from_str::<T>(&raw).map_err(|e| {
                AppError::from(format!("配置 '{}' 反序列化失败: {}", data_id, e))
            })?,
            None => T::default(),
        };

        let dc = DynamicConfig::new(initial, data_id);
        let dc2 = dc.clone();
        let did = data_id.to_string();
        let did_log = did.clone();
        let grp = self.cfg.group.clone();
        tokio::spawn(async move {
            cc.listen_config(
                &did,
                &grp,
                Box::new(move |raw| match serde_json::from_str::<T>(&raw) {
                    Ok(v) => dc2.update(v),
                    Err(e) => tracing::error!(data_id = %did_log, "配置热更新反序列化失败: {e}"),
                }),
            )
            .await;
        });
        Ok(dc)
    }

    // ── 服务注册能力 ────────────────────────────────────────────────────

    /// 注册 http/gRPC 端点（`take_endpoints` 取走端点），返回 `instance_id`
    pub async fn register_service(&self, endpoints: Vec<ServiceEndpoint>) -> RIE<String> {
        if endpoints.is_empty() {
            return Err(AppError::from("无可用端点，跳过服务注册"));
        }
        let instance_id = format!("{}-{}", self.cfg.service_name, fast_random_id());
        let instance = ServiceInstance {
            service_name: self.cfg.service_name.clone(),
            instance_id: instance_id.clone(),
            endpoints,
            healthy: true,
            metadata: Default::default(),
        };
        self.naming.register(&instance).await?;
        tracing::info!(
            service = %self.cfg.service_name,
            instance_id = %instance_id,
            "服务已注册到注册中心"
        );
        Ok(instance_id)
    }

    /// 注销服务实例（优雅关闭前调用）
    pub async fn deregister(&self, instance_id: &str) -> RIE<()> {
        self.naming.deregister(instance_id).await
    }
}

/// TOML 深合并：递归合并 table，`remote` 键覆盖 `local` 键；数组/标量直接覆盖
fn merge_toml(local: toml::Value, remote: toml::Value) -> toml::Value {
    match (local, remote) {
        (toml::Value::Table(mut lt), toml::Value::Table(rt)) => {
            for (k, v) in rt {
                let prev = lt.remove(&k).unwrap_or(toml::Value::Table(toml::map::Map::new()));
                lt.insert(k, merge_toml(prev, v));
            }
            toml::Value::Table(lt)
        }
        (_, remote) => remote,
    }
}

/// 快速生成唯一 ID（基于时间戳 + 原子计数器，避免引入 rand 依赖）
fn fast_random_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);

    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let rnd = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{:x}_{:04x}", ts, rnd)
}
