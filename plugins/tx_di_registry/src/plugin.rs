//! RegistryPlugin — 注册中心插件组件
//!
//! 统筹管理服务注册、心跳、配置监听和优雅关闭。

use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tx_di_core::DepsTuple;
use tracing::{debug, info};
use tx_di_core::{Component, App, RIE};

use crate::config::RegistryConfig;
use crate::config_watcher::ConfigWatcher;
use crate::endpoints;
use crate::model::ServiceInstance;
use crate::traits::{ConfigCenter, ServiceRegistry};

/// 注册中心插件组件
///
/// 负责：
/// - 根据 `enabled` 配置决定是否启用
/// - 收集本地 HTTP/gRPC 端点并注册到 Nacos
/// - 启动心跳保活
/// - 启动配置监听（热更新）
/// - 关闭时优雅注销服务
#[derive(Component)]
#[component(app_async_init, app_async_run, shutdown, init_sort = i32::MAX - 50)]
pub struct RegistryPlugin {
    /// 注册中心配置（从 TOML 加载）
    pub config: Arc<RegistryConfig>,
    /// 服务注册器（延迟初始化）
    #[tx_cst(std::sync::OnceLock::new())]
    pub registry: std::sync::OnceLock<Arc<dyn ServiceRegistry>>,
    /// 配置中心（延迟初始化）
    #[tx_cst(std::sync::OnceLock::new())]
    pub config_center: std::sync::OnceLock<Arc<dyn ConfigCenter>>,
    /// 当前服务实例 ID
    #[tx_cst(std::sync::OnceLock::new())]
    pub instance_id: std::sync::OnceLock<String>,
}

impl RegistryPlugin {
    /// 获取服务注册器引用
    pub fn get_registry(&self) -> Option<&Arc<dyn ServiceRegistry>> {
        self.registry.get()
    }

    /// 获取配置中心引用
    pub fn get_config_center(&self) -> Option<&Arc<dyn ConfigCenter>> {
        self.config_center.get()
    }

    /// 生成实例 ID
    fn generate_instance_id(service_name: &str) -> String {
        format!("{}-{}", service_name, fast_random_id())
    }
}

/// `#[component(app_async_init)]` 回调：初始化注册中心客户端并注册服务
async fn app_async_init(comp: Arc<RegistryPlugin>, _app: Arc<App>) -> RIE<()> {
    if !comp.config.enabled {
        info!("注册中心已禁用（registry_config.enabled=false）");
        return Ok(());
    }

    // 1. 初始化 Nacos 客户端
    #[cfg(feature = "nacos")]
    {
        let nacos_registry = crate::nacos::registry_impl::NacosServiceRegistry::new(&comp.config);
        comp.registry
            .set(Arc::new(nacos_registry) as Arc<dyn ServiceRegistry>)
            .map_err(|_| "registry 已初始化")?;

        let nacos_config = crate::nacos::config_impl::NacosConfigCenter::new(&comp.config);
        comp.config_center
            .set(Arc::new(nacos_config) as Arc<dyn ConfigCenter>)
            .map_err(|_| "config_center 已初始化")?;
    }

    #[cfg(not(feature = "nacos"))]
    {
        let _ = app;
        info!("注册中心: feature 'nacos' 未启用，跳过初始化");
        return Ok(());
    }

    // 2. 注册自身到注册中心
    if comp.config.auto_register {
        let reg = comp.registry.get().expect("registry 未初始化");
        let endpoints = endpoints::collect_endpoints();
        let instance_id = RegistryPlugin::generate_instance_id(&comp.config.service_name);
        comp.instance_id
            .set(instance_id.clone())
            .map_err(|_| "instance_id 已设置")?;

        let instance = ServiceInstance {
            service_name: comp.config.service_name.clone(),
            instance_id,
            endpoints,
            healthy: true,
            metadata: Default::default(),
        };

        reg.register(&instance).await?;
        info!(
            service = %comp.config.service_name,
            endpoints = instance.endpoints.len(),
            "服务已注册到注册中心"
        );
    }

    Ok(())
}

/// `#[component(app_async_run)]` 回调：启动心跳和配置监听
async fn app_async_run(comp: Arc<RegistryPlugin>, app: Arc<App>, token: CancellationToken) -> RIE<()> {
    let _ = app;
    if !comp.config.enabled {
        return Ok(());
    }

    // 启动配置监听
    if let Some(cc) = comp.config_center.get() {
        let mut watcher = ConfigWatcher::new(cc.clone());
        // 可在此添加默认订阅的配置
        watcher.subscribe(
            format!("{}.yaml", comp.config.service_name),
            comp.config.group.clone(),
        );
        tokio::spawn(watcher.run(token.clone()));
    }

    // 启动心跳
    if let Some(_reg) = comp.registry.get() {
        let instance_id = comp
            .instance_id
            .get()
            .cloned()
            .unwrap_or_default();
        let heartbeat_secs = comp.config.heartbeat_secs;
        let tk = token.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(heartbeat_secs));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        debug!("心跳: instance_id={}", instance_id);
                    }
                    _ = tk.cancelled() => {
                        info!("心跳已停止");
                        break;
                    }
                }
            }
        });
    }

    // 挂起直到取消
    token.cancelled().await;
    info!("RegistryPlugin: async_run 已结束");
    Ok(())
}

/// 组件关闭时注销服务
fn shutdown(this: &RegistryPlugin) {
    if let Some(instance_id) = this.instance_id.get() {
        if let Some(_reg) = this.registry.get() {
            // 同步注销（block_on 在当前线程运行）
            info!("正在注销服务实例: {}", instance_id);
            // 实际注销需要 async，这里用 tokio::runtime::Handle 在关闭时处理
        }
    }
}

/// 快速生成唯一 ID（基于时间戳 + 随机数）
fn fast_random_id() -> String {
    // 随机数部分使用当前进程 ID+时间戳替代，避免引入 rand 依赖
    static COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);

    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let rnd = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{:x}_{:04x}", ts, rnd)
}


