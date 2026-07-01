//! 生命周期管理 — BuildContext 和 App
//!
//! BuildContext 负责构建阶段：加载配置 → 拓扑排序 → 构建组件 → inner_init
//! App 负责运行阶段：init → async_init → async_run → shutdown

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::component::Component;
use crate::config::AppAllConfig;
use crate::registry::{ComponentMeta, COMPONENT_REGISTRY};
use crate::scope::Scope;
use crate::store::{CompRef, Store, TRAIT_IMPL_MAP};
use crate::topology::{all_metas, topo_sort};
use crate::{IE, RIE};

/// 内部上下文类型别名
pub type InnerContext = DashMap<TypeId, CompRef>;

/// 全局系统配置
static SYS_CONFIG: LazyLock<DashMap<String, String>> = LazyLock::new(DashMap::new);

/// 获取全局配置
pub fn get_sys_config(key: &str) -> Option<String> {
    SYS_CONFIG.get(key).map(|v| v.value().clone())
}

/// 设置全局配置
pub fn set_sys_config(key: &str, value: String) {
    SYS_CONFIG.insert(key.to_string(), value);
}

/// 配置路径 key
pub const CONFIG_PATH: &str = "config_path";

// ── BuildContext ──────────────────────────────────────────────────────────

/// 构建上下文 — 负责组件注册和初始化
pub struct BuildContext {
    store: Store,
    metas: Vec<&'static ComponentMeta>,
}

impl BuildContext {
    /// 创建一个新的 BuildContext（仅供内部使用 DashMap 的场景）
    pub fn inner_new(ctx: InnerContext) -> Self {
        BuildContext {
            store: Store::from_dashmap(ctx),
            metas: vec![],
        }
    }

    /// 创建一个新的 BuildContext
    ///
    /// # 参数
    ///
    /// * `config_path` - 可选的配置文件路径
    #[inline]
    pub fn new<P: Into<PathBuf>>(config_path: Option<P>) -> Self {
        let mut ctx = Self {
            store: Store::new(),
            metas: vec![],
        };

        // 加载配置文件并放入 store
        let app_configs = AppAllConfig::new(config_path);
        ctx.store.insert_cached(app_configs);

        // 自动扫描并注册所有组件
        ctx.auto_register_all();

        ctx
    }

    /// 自动注册所有通过 `#[derive(Component)]` 标记的组件
    fn auto_register_all(&mut self) {
        // 1. 填充 TRAIT_IMPL_MAP
        for meta in COMPONENT_REGISTRY.iter() {
            if !meta.trait_impls.is_empty() {
                for trait_fn in meta.impl_traits {
                    let trait_tid = trait_fn();
                    TRAIT_IMPL_MAP
                        .entry(trait_tid)
                        .or_default()
                        .extend(meta.trait_impls.to_vec());
                    debug!("组件 '{}' 实现了 trait {:?}", meta.name, trait_tid);
                }
            }
        }

        // 2. 拓扑排序
        let metas: Vec<&'static ComponentMeta> = COMPONENT_REGISTRY.iter().collect();
        let sorted_ids = topo_sort(&metas).unwrap_or_else(|e| {
            panic!("{}", e);
        });

        // 3. 按拓扑顺序注册工厂
        for tid in &sorted_ids {
            if let Some(meta) = metas.iter().find(|m| (m.type_id)() == *tid) {
                self.register_factory(meta);
                self.metas.push(meta);
            }
        }
    }

    /// 注册组件工厂
    ///
    /// - Singleton：立即调用工厂并缓存为 `CompRef::Cached`
    /// - Prototype：存为 `CompRef::Factory` 闭包
    fn register_factory(&mut self, meta: &ComponentMeta) {
        let type_id = (meta.type_id)();
        let scope = meta.scope;
        let factory = meta.factory;

        match scope {
            Scope::Singleton => {
                let instance = factory(&self.store);
                let arc: Arc<dyn Any + Send + Sync> = Arc::from(instance);
                self.store.inner().insert(type_id, CompRef::Cached(arc));
            }
            Scope::Prototype => {
                let closure =
                    move |store: &Store| -> Arc<dyn Any + Send + Sync> {
                        let boxed = factory(store);
                        Arc::from(boxed)
                    };
                self.store
                    .inner()
                    .insert(type_id, CompRef::Factory(Arc::new(closure)));
            }
        }
    }

    // ── 注入入口 ─────────────────────────────────────────────────────────

    /// 注入组件实例
    pub fn inject<T: Component>(&self) -> Arc<T> {
        self.store.inject_or_panic::<T>()
    }

    /// 尝试注入，失败返回 None
    pub fn try_inject<T: Component>(&self) -> Option<Arc<T>> {
        self.store.try_inject::<T>()
    }

    /// 获取 Store 引用
    pub fn store(&self) -> &Store {
        &self.store
    }

    // ── 调试辅助 ────────────────────────────────────────────────────────

    /// 已注册组件数量
    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// 是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// 打印所有已注册的组件（调试用）
    pub fn debug_registry() -> RIE<()> {
        let metas = all_metas();
        let id_to_idx: HashMap<TypeId, (usize, &str)> = metas
            .iter()
            .enumerate()
            .map(|(i, m)| ((m.type_id)(), (i, m.name)))
            .collect();
        let ans = topo_sort(&metas).map_err(|e| {
            IE::Internal(anyhow::anyhow!("{}", e))
        })?;

        debug!("组件注册表（拓扑排序后）：");
        debug!("{:20} {:10} deps", "name", "scope");
        for tid in ans.iter() {
            let meta = metas[id_to_idx
                .get(tid)
                .ok_or_else(|| IE::Internal(anyhow::anyhow!("RegistryError")))?
                .0];
            let dep_names: Vec<&str> = meta
                .dep_type_ids
                .iter()
                .map(|dep_fn| {
                    COMPONENT_REGISTRY
                        .iter()
                        .find(|m| (m.type_id)() == dep_fn())
                        .map(|m| m.name)
                        .unwrap_or("unknown")
                })
                .collect();
            debug!(
                "{:20} {:10} [{}]",
                meta.name,
                format!("{:?}", meta.scope),
                dep_names.join(", ")
            )
        }
        Ok(())
    }

    // ── 构建 App ────────────────────────────────────────────────────────

    /// 构建 App 实例，将 store 转移到 App
    pub fn build(mut self) -> RIE<App> {
        let shutdown_token = CancellationToken::new();
        let store = std::mem::replace(&mut self.store, Store::new());
        let metas = std::mem::take(&mut self.metas);
        Ok(App {
            store,
            metas,
            shutdown_token,
            task_handle: RwLock::new(None),
        })
    }

    /// 构建 App 并运行
    pub async fn build_and_run(self) -> RIE<()> {
        let app = self.build()?;
        let arc_app = Arc::new(app);
        App::run(arc_app.clone(), arc_app.shutdown_token.clone()).await
    }
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new::<PathBuf>(None)
    }
}

// ── App ───────────────────────────────────────────────────────────────────

/// 运行时 App — 持有所有已初始化的组件
pub struct App {
    pub store: Store,
    pub metas: Vec<&'static ComponentMeta>,
    pub shutdown_token: CancellationToken,
    pub task_handle: RwLock<Option<JoinHandle<()>>>,
}

impl App {
    /// 获取组件实例
    pub fn inject<T: Component>(&self) -> Arc<T> {
        self.store.inject_or_panic::<T>()
    }

    /// 尝试获取组件，失败返回 None
    pub fn try_inject<T: Component>(&self) -> Option<Arc<T>> {
        self.store.try_inject::<T>()
    }

    /// 获取组件总数
    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// 检查 App 是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// 获取 Store 引用
    pub fn store(&self) -> &Store {
        &self.store
    }

    // ── 生命周期执行 ─────────────────────────────────────────────────────

    /// 同步初始化阶段：按 init_sort 顺序调用所有组件的 init()
    fn init(app: &Arc<App>) -> RIE<()> {
        // 按 init_sort 排序（值越小越先执行）
        let mut sorted_metas: Vec<&ComponentMeta> = app.metas.clone();
        sorted_metas.sort_by_key(|m| (m.init_sort_fn)());

        for meta in &sorted_metas {
            debug!("[di] init: {}", meta.name);
            (meta.init_fn)(app)?;
        }
        Ok(())
    }

    /// 异步初始化阶段：按 init_sort 顺序调用所有组件的 async_init()
    async fn async_init(app: &Arc<App>) -> RIE<()> {
        let mut sorted_metas: Vec<&ComponentMeta> = app.metas.clone();
        sorted_metas.sort_by_key(|m| (m.init_sort_fn)());

        for meta in &sorted_metas {
            debug!("[di] async_init: {}", meta.name);
            (meta.async_init_fn)(app).await?;
        }
        Ok(())
    }

    /// 并行运行所有组件的 async_run()
    async fn comp_run(app: Arc<App>, token: CancellationToken) -> RIE<()> {
        let mut handles = Vec::new();

        // 先收集所有 meta 引用，避免借用 app.metas
        let metas: Vec<&'static ComponentMeta> = app.metas.clone();
        for meta in metas {
            let app_clone = app.clone();
            let token_clone = token.clone();
            let name = meta.name;
            debug!("[di] async_run spawn: {}", name);

            let handle = tokio::spawn(async move {
                if let Err(e) = (meta.async_run_fn)(&app_clone, token_clone).await {
                    tracing::error!("[di] 组件 '{}' async_run 失败: {:?}", name, e);
                }
            });
            handles.push(handle);
        }
        // 等待所有后台任务完成（或被 cancel）
        for handle in handles {
            let _ = handle.await;
        }
        Ok(())
    }

    /// 运行 App（init → async_init → async_run）
    async fn run(app: Arc<App>, token: CancellationToken) -> RIE<()> {
        App::init(&app)?;
        App::async_init(&app).await?;
        App::comp_run(app, token).await?;
        Ok(())
    }

    /// 异步运行 App，返回 Arc<App>
    pub async fn ins_run(self) -> RIE<Arc<App>> {
        let app = Arc::new(App {
            store: self.store,
            metas: self.metas,
            shutdown_token: self.shutdown_token,
            task_handle: self.task_handle,
        });

        let app_clone = app.clone();
        let app_handler = tokio::spawn(async move {
            if let Err(e) = App::run(app_clone.clone(), app_clone.shutdown_token.clone()).await {
                tracing::error!("[di] App 运行失败: {:?}", e);
                std::process::exit(1);
            }
        });

        {
            let mut guard = app.task_handle.write().await;
            *guard = Some(app_handler);
        }

        Ok(app)
    }

    /// 优雅关闭所有组件
    pub async fn shutdown(&self) {
        let metas: Vec<&ComponentMeta> = self.metas.clone();
        // 逆序关闭（后注册的先关闭）
        for meta in metas.iter().rev() {
            debug!("[di] shutdown: {}", meta.name);
            (meta.shutdown_fn)(&self.store);
        }
    }

    /// 等待退出信号并优雅关闭
    pub async fn waiting_exit(&self) {
        App::wait_for_exit_signal().await;
        let start = Instant::now();
        info!("正在等待退出...");
        self.shutdown_token.cancel();

        if let Some(handle) = self.task_handle.write().await.take() {
            match tokio::time::timeout(std::time::Duration::from_secs(5), handle).await {
                Ok(Ok(())) => {
                    info!("后台任务已正常关闭");
                }
                Ok(Err(e)) => {
                    tracing::error!("后台任务退出时发生错误: {:?}", e);
                }
                Err(_) => {
                    tracing::warn!("后台任务关闭超时（5秒），强制退出");
                }
            }
        }

        // 优雅关闭所有组件
        self.shutdown().await;

        info!("app 已退出，耗时: {:?}", start.elapsed());
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    /// 跨平台等待退出信号
    async fn wait_for_exit_signal() {
        #[cfg(unix)]
        {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("无法注册 SIGTERM 处理器");
            let mut sighup = signal::unix::signal(signal::unix::SignalKind::hangup())
                .expect("无法注册 SIGHUP 处理器");
            tokio::select! {
                _ = signal::ctrl_c() => {},
                _ = sigterm.recv() => {},
                _ = sighup.recv() => {},
            }
        }
        #[cfg(windows)]
        {
            use tokio::signal::windows;
            let ctrl_c = signal::ctrl_c();
            let mut ctrl_break = windows::ctrl_break().expect("无法注册 Ctrl+Break 处理器");
            let mut ctrl_close = windows::ctrl_close().expect("无法注册 Ctrl+Close 处理器");
            let mut ctrl_shutdown =
                windows::ctrl_shutdown().expect("无法注册 Ctrl+Shutdown 处理器");
            tokio::select! {
                _ = ctrl_c => {},
                _ = ctrl_break.recv() => {},
                _ = ctrl_close.recv() => {},
                _ = ctrl_shutdown.recv() => {},
            }
        }
        #[cfg(all(not(unix), not(windows)))]
        {
            let _ = signal::ctrl_c().await;
        }
    }
}
