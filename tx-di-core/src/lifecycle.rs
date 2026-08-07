//! 生命周期管理 — BuildContext 和 App
//!
//! BuildContext 负责构建阶段：加载配置 → 拓扑排序 → 构建组件 → inner_init
//! App 负责运行阶段：init → async_init → async_run → shutdown

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dashmap::DashMap;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::component::Component;
use crate::config::AppAllConfig;
use crate::error::{AppError, DiErr};
use crate::registry::{ComponentMeta, COMPONENT_REGISTRY};
use crate::scope::Scope;
use crate::store::{CompRef, Store, TraitImplEntry};
use crate::topology::{all_metas, topo_sort};
use crate::RIE;

/// 内部上下文类型别名
pub type InnerContext = DashMap<TypeId, CompRef>;

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
    pub fn new<P: Into<PathBuf>>(config_path: Option<P>) -> RIE<Self> {
        let mut ctx = Self {
            store: Store::new(),
            metas: vec![],
        };

        // 加载配置文件并放入 store
        let app_configs = AppAllConfig::new(config_path)?;
        ctx.store.insert_cached(app_configs);

        // 自动扫描并注册所有组件
        ctx.auto_register_all()?;

        Ok(ctx)
    }

    /// 从内存配置创建 BuildContext（配置中心拉取场景）
    ///
    /// 典型用法（配合 `tx_di_nacos`，配置中心作为配置源，改配置 → 优雅重启 → 生效）：
    ///
    /// ```rust,ignore
    /// // 1. 启动早期（BuildContext 之前）连接配置中心并拉取远程配置
    /// let client = tx_di_nacos::NacosClient::connect(&bootstrap).await?;
    /// let remote = client.pull_config("tx-admin.toml").await?;
    /// // 2. 合并（远程覆盖本地 bootstrap）
    /// let merged = client.merge_config(local_toml, remote)?;
    /// // 3. 用合并后的配置构建应用（组件按新配置初始化）
    /// let ctx = BuildContext::with_config(merged)?;
    /// ```
    pub fn with_config(toml_value: toml::Value) -> RIE<Self> {
        let mut ctx = Self {
            store: Store::new(),
            metas: vec![],
        };

        let app_configs = AppAllConfig::from_toml_value(toml_value)?;
        ctx.store.insert_cached(app_configs);

        ctx.auto_register_all()?;

        Ok(ctx)
    }

    /// 自动注册所有通过 `#[derive(Component)]` 标记的组件
    fn auto_register_all(&mut self) -> RIE<()> {
        // 1. 填充 trait_impls（每个 Store 拥有独立的 trait 映射，无全局污染）
        for meta in COMPONENT_REGISTRY.iter() {
            if !meta.trait_impls.is_empty() {
                for trait_fn in meta.impl_traits {
                    let trait_tid = trait_fn();
                    self.store
                        .trait_impls
                        .entry(trait_tid)
                        .or_default()
                        .extend(meta.trait_impls.to_vec());
                    debug!("组件 '{}' 实现了 trait {:?}", meta.name, trait_tid);
                }
            }
        }

        // 2. 拓扑排序
        let metas: Vec<&'static ComponentMeta> = COMPONENT_REGISTRY.iter().collect();
        let sorted_ids = topo_sort(&metas, &self.store.trait_impls)?;

        // 3. 按拓扑顺序注册工厂（预建 HashMap 避免 O(n²)）
        let meta_map: std::collections::HashMap<TypeId, &ComponentMeta> = metas
            .iter()
            .map(|m| ((m.type_id)(), *m))
            .collect();
        for tid in &sorted_ids {
            if let Some(meta) = meta_map.get(tid) {
                self.register_factory(meta);
                self.metas.push(meta);
            }
        }

        Ok(())
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
                        let arc: Arc<dyn Any + Send + Sync> = Arc::from(boxed);
                        // 追踪 Prototype 实例，以便 shutdown 时通知
                        store.track_prototype_raw(&arc);
                        arc
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

        // 构建临时 trait_impls 用于拓扑排序（无 Store 环境）
        let temp_trait_impls: DashMap<TypeId, Vec<TraitImplEntry>> = DashMap::new();
        for meta in COMPONENT_REGISTRY.iter() {
            if !meta.trait_impls.is_empty() {
                for trait_fn in meta.impl_traits {
                    let trait_tid = trait_fn();
                    temp_trait_impls
                        .entry(trait_tid)
                        .or_default()
                        .extend(meta.trait_impls.to_vec());
                }
            }
        }

        let ans = topo_sort(&metas, &temp_trait_impls)?;

        debug!("组件注册表（拓扑排序后）：");
        debug!("{:20} {:10} deps", "name", "scope");
        for tid in ans.iter() {
            let meta = metas[id_to_idx
                .get(tid)
                .ok_or_else(|| AppError::with_context(DiErr::RegistryError, "RegistryError"))?
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
            shutdown_called: AtomicBool::new(false),
            shutdown_timeout_secs: 5,
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
    /// 使用默认配置路径创建 BuildContext。测试/演示用途，失败时 panic。
    fn default() -> Self {
        Self::new::<PathBuf>(None).expect("BuildContext::default() 失败")
    }
}

// ── App ───────────────────────────────────────────────────────────────────

/// 运行时 App — 持有所有已初始化的组件
pub struct App {
    pub store: Store,
    pub metas: Vec<&'static ComponentMeta>,
    pub shutdown_token: CancellationToken,
    pub task_handle: RwLock<Option<JoinHandle<()>>>,
    /// 幂等门闩：shutdown 只执行一次
    shutdown_called: AtomicBool,
    /// 后台任务关闭超时（秒），默认 5
    pub shutdown_timeout_secs: u64,
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

    /// 同步初始化阶段：按已排序顺序（拓扑序 + init_sort）调用所有组件的 init()
    fn init(app: &Arc<App>) -> RIE<()> {
        // App.metas 已在 BuildContext::auto_register_all 中按 topo_sort(init_sort) 排序，
        // 同时满足依赖关系和 init_sort 优先级，无需重复排序
        for meta in &app.metas {
            debug!("[di] init: {}", meta.name);
            (meta.init_fn)(app)?;
        }
        Ok(())
    }

    /// 异步初始化阶段：按拓扑顺序串行调用所有组件的 async_init()
    ///
    /// metas 已按拓扑序排列，依赖者的 async_init（如建立 DB 连接）先于消费者执行。
    /// TODO: 分层并行 — 同拓扑深度的组件可并发 exec，但需额外依赖关系分析。
    async fn async_init(app: &Arc<App>) -> RIE<()> {
        for meta in &app.metas {
            debug!("[di] async_init: {}", meta.name);
            (meta.async_init_fn)(app).await?;
        }
        Ok(())
    }

    /// 并行运行所有组件的 async_run()，跳过未覆写回调的组件
    async fn comp_run(app: Arc<App>, token: CancellationToken) -> RIE<()> {
        let mut handles = Vec::new();

        let metas: Vec<&'static ComponentMeta> = app.metas.clone();
        for meta in metas {
            if !meta.has_async_run {
                continue; // 未覆写 async_run，跳过
            }
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
    ///
    /// 初始化阶段（`init` + `async_init`）会在返回 `Arc<App>` **之前**同步完成，
    /// 以确保组件完全就绪（如 AOP 拦截链注册、跨组件协作初始化）后再交予调用方使用。
    /// 仅长期运行的后台任务（`async_run`）放入独立 task 中持续运行，
    /// 直到 `shutdown_token` 触发才退出。
    pub async fn ins_run(self) -> RIE<Arc<App>> {
        let app = Arc::new(App {
            store: self.store,
            metas: self.metas,
            shutdown_token: self.shutdown_token,
            task_handle: self.task_handle,
            shutdown_called: AtomicBool::new(false),
            shutdown_timeout_secs: 5,
        });

        // 初始化阶段必须先完成，否则调用方立即访问组件时会因尚未就绪而失败
        // （例如被 #[component(intercept(...))] 标记的组件其拦截链在 init 中注册）。
        App::init(&app)?;
        App::async_init(&app).await?;

        // 仅长期后台任务（async_run）放入独立 task 运行，直到 token 触发退出
        let app_clone = app.clone();
        let app_handler = tokio::spawn(async move {
            if let Err(e) = App::comp_run(app_clone.clone(), app_clone.shutdown_token.clone()).await {
                tracing::error!("[di] App 运行失败: {:?}，将执行 shutdown", e);
                app_clone.shutdown().await;
            }
        });

        {
            let mut guard = app.task_handle.write().await;
            *guard = Some(app_handler);
        }

        Ok(app)
    }

    /// 优雅关闭所有组件（幂等：多次调用只执行一次）
    pub async fn shutdown(&self) {
        if self.shutdown_called.swap(true, Ordering::SeqCst) {
            return; // 已执行过 shutdown，跳过
        }
        let metas: Vec<&ComponentMeta> = self.metas.clone();
        for meta in metas.iter().rev() {
            debug!("[di] shutdown: {}", meta.name);
            (meta.shutdown_fn)(&self.store);
        }
        self.store.shutdown_prototypes();
    }

    /// 等待退出信号并优雅关闭（等价于 `wait_exit_signal().await` + `graceful_shutdown().await`）
    pub async fn waiting_exit(&self) {
        App::wait_exit_signal().await;
        let _ = self.graceful_shutdown().await;
    }

    /// 优雅关闭当前实例（**不退出进程**，可再次启动新 App）
    ///
    /// 用于配置中心「配置变更 → 优雅重启」场景：外层循环调用本方法关闭旧实例，
    /// 然后用新配置重新构建并启动。幂等（shutdown 只执行一次）。
    pub async fn graceful_shutdown(&self) -> RIE<()> {
        let start = Instant::now();
        info!("正在优雅关闭...");
        self.shutdown_token.cancel();

        if let Some(handle) = self.task_handle.write().await.take() {
            let timeout = std::time::Duration::from_secs(self.shutdown_timeout_secs);
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(())) => {
                    info!("后台任务已正常关闭");
                }
                Ok(Err(e)) => {
                    tracing::error!("后台任务退出时发生错误: {:?}", e);
                }
                Err(_) => {
                    tracing::warn!("后台任务关闭超时（{}秒），强制退出", self.shutdown_timeout_secs);
                }
            }
        }

        // 优雅关闭所有组件（幂等）
        self.shutdown().await;

        info!("app 已关闭，耗时: {:?}", start.elapsed());
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }

    /// 跨平台等待退出信号（Ctrl+C / SIGTERM / SIGHUP）
    pub async fn wait_exit_signal() {
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
