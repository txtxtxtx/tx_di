// ─────────────────────────────────────────────────────────────────────────────
// 4. BuildContext
// ─────────────────────────────────────────────────────────────────────────────

pub mod common;
pub mod comp;
pub mod scopes;

use crate::di::comp::config::AppAllConfig;
use crate::{
    COMPONENT_REGISTRY, CompRef, ComponentDescriptor, ComponentMeta, IE, RIE, Scope, topo_sort,
};
use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

/// 构建上下文
pub struct BuildContext {
    /// TypeId → CompRef（使用 DashMap 支持并发访问）
    store: DashMap<TypeId, CompRef>,
    metas: Vec<&'static ComponentMeta>,
}

impl crate::BuildContext {
    /// 创建一个新的 BuildContext。
    ///
    /// # 参数
    ///
    /// * `config_path` - 可选的配置文件路径。
    ///
    /// # 配置文件格式 (TOML)
    /// ```
    #[inline]
    pub fn new<P: Into<PathBuf>>(config_path: Option<P>) -> Self {
        let mut ctx = Self {
            store: DashMap::new(),
            metas: vec![],
        };
        // 加载配置文件并放入全局上下文
        let app_configs = AppAllConfig::new(config_path);
        ctx.store.insert(
            TypeId::of::<AppAllConfig>(),
            CompRef::Cached(Arc::new(app_configs)),
        );

        // 自动扫描并注册所有组件（通过拓扑排序）
        ctx.auto_register_all();

        ctx
    }

    /// 自动注册所有通过 #[tx_comp] 标记的组件
    fn auto_register_all(&mut self) {
        let metas: Vec<&ComponentMeta> = COMPONENT_REGISTRY.iter().collect();
        let sorted_ids = topo_sort(&metas);

        for tid in &sorted_ids {
            if let Some(meta) = metas.iter().find(|m| (m.type_id)() == *tid) {
                if let Some(factory_fn) = meta.factory_fn {
                    self.register_factory_boxed((meta.type_id)(), meta.scope, factory_fn);
                }
                self.metas.push(meta);
            }
        }
    }

    // ── 注册 ─────────────────────────────────────────────────────────────────

    /// 注册组件的工厂函数。
    ///
    /// `factory` 返回 `Box<T>`：
    /// - Singleton：立即调用，存入 `Box<Arc<T>>`
    /// - Prototype：用 Arc<dyn Fn> 包装，闭包每次调用时构造新实例
    pub fn register_factory<T: Any + Send + Sync + 'static>(
        &mut self,
        scope: Scope,
        factory: fn(&mut crate::BuildContext) -> Box<T>,
    ) {
        match scope {
            Scope::Singleton => {
                // 单例：立即调用 factory，构造 Arc<T> 后缓存
                let instance: Arc<T> = Arc::new(*factory(self));
                self.store
                    .insert(TypeId::of::<T>(), CompRef::Cached(instance));
            }
            Scope::Prototype => {
                // 原型：存闭包，每次调用时构造新实例
                let factory_fn = factory;
                let closure = move |ctx: &mut crate::BuildContext| -> Arc<dyn Any + Send + Sync> {
                    let boxed: Box<T> = (factory_fn)(ctx);
                    Arc::new(*boxed) as Arc<dyn Any + Send + Sync>
                };
                self.store
                    .insert(TypeId::of::<T>(), CompRef::Factory(Arc::new(closure)));
            }
        }
    }

    /// 注册已擦除类型的工厂函数（用于从 COMPONENT_REGISTRY 批量注册）。
    pub fn register_factory_boxed(
        &mut self,
        type_id: TypeId,
        scope: Scope,
        factory: fn(&mut crate::BuildContext) -> Box<dyn Any + Send + Sync>,
    ) {
        match scope {
            Scope::Singleton => {
                let instance: Box<dyn Any + Send + Sync> = factory(self);
                let arc: Arc<dyn Any + Send + Sync> = Arc::from(instance);
                self.store.insert(type_id, CompRef::Cached(arc));
            }
            Scope::Prototype => {
                let factory_fn = factory;
                let closure = move |ctx: &mut crate::BuildContext| -> Arc<dyn Any + Send + Sync> {
                    let boxed: Box<dyn Any + Send + Sync> = (factory_fn)(ctx);
                    Arc::from(boxed)
                };
                self.store
                    .insert(type_id, CompRef::Factory(Arc::new(closure)));
            }
        }
    }

    // ── 统一注入入口 ─────────────────────────────────────────────────────────

    /// 统一注入入口。根据被注入组件 T 的 scope 自动选择：
    ///
    /// 注意：scope 来自被注入者（T 自己的 SCOPE），而非调用者的 scope。
    pub fn inject<T: Any + Send + Sync + 'static + ComponentDescriptor>(&mut self) -> Arc<T> {
        let tid = TypeId::of::<T>();
        // 直接用编译期常量，避免在构建过程中动态查询 registry
        let scope = <T as ComponentDescriptor>::SCOPE;

        match scope {
            Scope::Singleton => self.inject_singleton::<T>(tid),
            Scope::Prototype => self.inject_prototype::<T>(tid),
        }
    }

    /// 获取单例组件的不可变引用版本。
    ///
    /// 此方法不需要 `&mut self`，但只能用于 Singleton 作用域的组件。
    /// 如果尝试获取 Prototype 组件，将会 panic。
    ///
    /// # Panics
    ///
    /// - 如果组件是 Prototype 作用域
    /// - 如果组件未注册
    /// - 如果类型转换失败
    pub fn get<T: ComponentDescriptor>(&self) -> Arc<T> {
        let tid = TypeId::of::<T>();
        let scope = <T as ComponentDescriptor>::SCOPE;

        match scope {
            Scope::Singleton => self.inject_singleton::<T>(tid),
            Scope::Prototype => {
                panic!(
                    "[di] get::<{}> 不能用于 Prototype 组件，请使用 inject() 方法",
                    std::any::type_name::<T>()
                )
            }
        }
    }

    /// 通过 `Arc<BuildContext>` 获取已缓存的单例组件（无需 `ComponentDescriptor` 约束）。
    ///
    /// 适用于 axum handler 等场景：持有 `Arc<BuildContext>` 时只能拿 `&self`，
    /// 而 `inject` 需要 `&mut self`。此方法绕过该限制，直接从 store 读取已缓存的 Arc。
    ///
    /// # Panics
    ///
    /// - 组件未注册或未缓存（Prototype 组件无法通过此方法获取）
    /// - downcast 失败
    pub fn get_singleton<T: Any + Send + Sync + 'static>(&self) -> Arc<T> {
        self.inject_singleton::<T>(TypeId::of::<T>())
    }

    /// 尝试获取已缓存的单例组件，未找到时返回 `None`（不 panic）。
    ///
    /// 适合在 axum handler 等不能 panic 的场景中使用。
    pub fn try_get_singleton<T: Any + Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let tid = TypeId::of::<T>();
        self.store.get(&tid).and_then(|entry| match &*entry {
            CompRef::Cached(any_arc) => any_arc.clone().downcast::<T>().ok(),
            CompRef::Factory(_) => None,
        })
    }

    /// 注入单例：factory 只调用一次，之后返回缓存的 Arc。
    fn inject_singleton<T: Any + Send + Sync + 'static>(&self, tid: TypeId) -> Arc<T> {
        self.store
            .get(&tid)
            .map(|entry| match &*entry {
                CompRef::Cached(any_arc) => any_arc.clone(),
                CompRef::Factory(_) => {
                    panic!(
                        "[di] inject_singleton::<{}> 错误：组件注册为 Prototype",
                        std::any::type_name::<T>()
                    )
                }
            })
            .unwrap_or_else(|| {
                panic!(
                    "[di] inject::<{}> 未找到，请确认该组件已注册（使用 #[tx_comp] 注解）",
                    std::any::type_name::<T>()
                )
            })
            .downcast::<T>()
            .unwrap_or_else(|_| {
                panic!(
                    "[di] inject singleton downcast 失败：{}",
                    std::any::type_name::<T>()
                )
            })
    }

    /// 注入原型：factory 每次都调用，构造新实例。
    fn inject_prototype<T: Any + Send + Sync + 'static>(&mut self, tid: TypeId) -> Arc<T> {
        // 1. 先把 factory_arc 从 Ref 中提取出来
        let factory_arc = self
            .store
            .get(&tid)
            .map(|entry| match &*entry {
                CompRef::Factory(f) => Some(f.clone()),
                _ => None,
            })
            .flatten()
            .unwrap_or_else(|| panic!("[di] inject::<{}> 未找到", std::any::type_name::<T>()));
        // 此时 Ref 已经 dropped，self 不再被不可变借用

        // 3. 现在可以安全调用 factory_arc(self)
        factory_arc(self)
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("[di] downcast 失败：{}", std::any::type_name::<T>()))
    }

    /// 从上下文中取出并移除单例（所有权）。
    pub fn take<T: Any + Send + Sync + 'static>(&mut self) -> RIE<T> {
        let name = std::any::type_name::<T>();
        let entry = self
            .store
            .remove(&TypeId::of::<T>())
            .ok_or_else(|| IE::Other(format!("取出组件失败,未找到该组件:{name}")))?
            .1;

        match entry {
            CompRef::Cached(any_arc) => {
                let arc_t: Arc<T> = any_arc.downcast::<T>().unwrap_or_else(|_| {
                    panic!("[di] take downcast 失败：{}", std::any::type_name::<T>())
                });
                Arc::try_unwrap(arc_t)
                    .map_err(|_| IE::Other(format!("取出组件失败,无法获取所有权:{name}")))
            }
            _ => Err(IE::Other(format!("取出组件失败,该组件不是单例:{name}"))),
        }
    }

    // ── 调试辅助 ────────────────────────────────────────────────────────────

    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// 打印所有已注册的组件（调试用）
    pub fn debug_registry() -> RIE<()> {
        let metas: Vec<&ComponentMeta> = COMPONENT_REGISTRY.iter().collect();
        let id_to_idx: HashMap<TypeId, (usize, &str)> = metas
            .iter()
            .enumerate()
            .map(|(i, m)| ((m.type_id)(), (i, m.name)))
            .collect();
        let ans = topo_sort(&metas);

        debug!("组件注册表：");
        debug!("{:20} scope      deps", "name");
        for meta in ans.iter() {
            let meta = metas[id_to_idx
                .get(meta)
                .ok_or_else(|| IE::Other("组件注册表错误".to_string()))?
                .0];
            let dep_names: Vec<&str> = meta
                .deps
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
                "{:20} {:?}  [{}]",
                meta.name,
                meta.scope,
                dep_names.join(", ")
            )
        }
        Ok(())
    }

    /// 构建 App 实例，完成所有初始化并将 store 转移
    ///
    /// 该方法会按顺序执行：
    /// 1. 同步初始化：调用所有组件的 init() 函数
    /// 2. 异步初始化：调用所有组件的 async_init() 函数
    /// 3. 转移所有权：使用 std::mem::replace 将 self.store 移动到 App
    ///
    /// # 返回值
    ///
    /// 返回包含所有已初始化组件的 App 实例
    ///
    /// # 注意
    ///
    /// 调用此方法后，self.store 会被替换为空的 DashMap，
    /// 此 BuildContext 实例不应再继续使用。
    pub fn build(mut self) -> RIE<App> {
        let shutdown_token = CancellationToken::new();
        // 使用 std::mem::replace 将 self.store 替换为空的 DashMap，取出原来的 store
        // 这样可以在不获取 self 所有权的情况下，将 store 移动出去
        let store = std::mem::replace(&mut self.store, DashMap::new());
        let metas: Vec<&ComponentMeta> = std::mem::replace(&mut self.metas, Vec::new());
        Ok(App {
            store,
            metas,
            shutdown_token,
            task_handle: RwLock::new(None),
        })
    }
    /// 构建 App 运行
    pub async fn build_and_run(self) -> RIE<()> {
        let app = self.build()?;
        let arc_app = Arc::new(app);
        App::run(arc_app.clone(), arc_app.shutdown_token.clone()).await
    }
}

impl Default for crate::BuildContext {
    fn default() -> Self {
        Self::new::<PathBuf>(None)
    }
}

/// 固定的组件上下文
pub struct App {
    pub store: DashMap<TypeId, CompRef>,
    pub metas: Vec<&'static ComponentMeta>,
    shutdown_token: CancellationToken,
    task_handle: RwLock<Option<JoinHandle<()>>>,
}

impl App {
    /// 获取单例,原型再固定期就不能直接获取了
    pub fn inject<T: Any + Send + Sync + 'static + ComponentDescriptor>(&self) -> Arc<T> {
        let tid = TypeId::of::<T>();
        self.store
            .get(&tid)
            .map(|entry| match &*entry {
                CompRef::Cached(any_arc) => any_arc.clone(),
                CompRef::Factory(_) => {
                    panic!(
                        "[di] inject_singleton::<{}> 错误：组件注册为 Prototype",
                        std::any::type_name::<T>()
                    )
                }
            })
            .unwrap_or_else(|| {
                panic!(
                    "[di] inject::<{}> 未找到，请确认该组件已注册（使用 #[tx_comp] 注解）",
                    std::any::type_name::<T>()
                )
            })
            .downcast::<T>()
            .unwrap_or_else(|_| {
                panic!(
                    "[di] inject singleton downcast 失败：{}",
                    std::any::type_name::<T>()
                )
            })
    }

    /// 尝试获取单例组件，失败时返回 None
    ///
    /// 该方法用于安全地获取已缓存的单例组件，不会 panic。
    /// 适用于运行时可能不存在某些组件的场景。
    ///
    /// # 返回值
    ///
    /// - `Some(Arc<T>)`: 组件存在且类型匹配
    /// - `None`: 组件未注册、是 Prototype 类型、或类型转换失败
    ///
    /// # 注意
    ///
    /// - 只能获取 Singleton 类型的组件，Prototype 组件始终返回 None
    /// - 不进行类型检查的 panic，失败时静默返回 None
    pub fn try_inject<T: Any + Send + Sync + 'static + ComponentDescriptor>(
        &self,
    ) -> Option<Arc<T>> {
        let tid = TypeId::of::<T>();
        self.store
            .get(&tid)
            .map(|entry| match &*entry {
                // 单例：克隆 Arc 引用
                CompRef::Cached(any_arc) => Some(any_arc.clone()),
                // 原型组件：App 阶段不支持动态创建，返回 None
                CompRef::Factory(_) => None,
            })
            .flatten()
            .and_then(|any_arc| {
                // 尝试向下转型为目标类型，失败则返回 None
                any_arc.downcast::<T>().ok()
            })
    }

    /// 获取组件的总数
    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// 检查App是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    fn init(app: Arc<App>) -> RIE<()> {
        let mut metas: Vec<&ComponentMeta> = COMPONENT_REGISTRY
            .iter()
            .filter(|m| m.async_init_fn.is_some())
            .collect();

        metas.sort_by_key(|m| (m.init_sort_fn)());

        for meta in metas {
            if let Some(init_fn) = meta.init_fn {
                // 直接调函数指针，传入 &mut self（DashMap 的 owner）
                init_fn(app.clone())?;
            }
        }
        Ok(())
    }

    async fn async_init(app: Arc<App>, token: CancellationToken) -> RIE<()> {
        let mut metas: Vec<&ComponentMeta> = COMPONENT_REGISTRY
            .iter()
            .filter(|m| m.async_init_fn.is_some())
            .collect();

        metas.sort_by_key(|m| (m.init_sort_fn)());

        // 收集所有异步初始化任务并并行执行
        let futures: Vec<_> = metas
            .iter()
            .filter_map(|meta| {
                meta.async_init_fn
                    .map(|init_fn| init_fn(app.clone(), token.clone()))
            })
            .collect();

        if futures.is_empty() {
            return Ok(());
        }

        // 使用 tokio::spawn 并行执行所有任务
        let handles: Vec<_> = futures
            .into_iter()
            .map(|future| tokio::spawn(future))
            .collect();

        // 等待所有任务完成并收集结果
        let mut errors = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(IE::Other(format!("Task panicked: {}", e))),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.remove(0)) // 返回第一个错误
        }
    }

    /// 阻塞运行 App
    async fn run(app: Arc<App>, token: CancellationToken) -> RIE<()> {
        App::init(app.clone())?;
        App::async_init(app, token).await?;
        Ok(())
    }

    /// 通过 app 实例异步运行 App，并返回 Arc<App>
    ///
    /// 此方法会消耗 self，将初始化过程放到 tokio::spawn 中执行，避免阻塞当前线程。
    /// 初始化完成后，返回 Arc<App> 供后续使用。
    ///
    /// # 注意
    /// - 初始化在后台异步执行，调用方需要确保在使用 App 之前初始化已完成
    /// - 如果需要等待初始化完成，请使用 `ins_run_blocking()` 或手动等待返回的 future
    pub async fn ins_run(self) -> RIE<Arc<App>> {
        // 创建 Arc<App> 用于初始化
        let app = Arc::new(App {
            store: self.store,
            metas: self.metas,
            shutdown_token: self.shutdown_token,
            task_handle: self.task_handle,
        });

        let app_clone = app.clone();

        // 在后台 spawn 初始化任务
        let app_handler = tokio::spawn(async move {
            if let Err(e) = App::run(app_clone.clone(), app_clone.shutdown_token.clone()).await {
                tracing::error!("[di] App 初始化失败: {:?}", e);
            }
        });

        {
            // 直接 await 获取写锁并设置句柄（现在是 async 方法，可以直接 await）
            let mut guard = app.task_handle.write().await;
            *guard = Some(app_handler);
        }

        Ok(app)
    }

    pub async fn waiting_exit(&self) {
        // 等待 Ctrl+C 信号
        App::wait_for_exit_signal().await;
        // 统计退出耗时
        let start = Instant::now();
        info!("正在等待退出...");
        self.shutdown_token.cancel();
        // 等待后台任务真正结束 ➜ 依赖 task_handle
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
        } else {
            tracing::warn!("未找到后台任务句柄");
        }
        info!("app 已安全退出，耗时: {:?} ", start.elapsed());
        // 确保日志被 flush 到输出
        // 短暂休眠以确保日志被 flush
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    /// 跨平台等待退出信号：Linux/macOS 等 Ctrl+C 和 SIGTERM，Windows 只等 Ctrl+C
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
        #[cfg(not(unix))]
        {
            let _ = signal::ctrl_c().await;
        }
    }
}
