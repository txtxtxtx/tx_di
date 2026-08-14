//! Store — 类型擦除的组件存储
//!
//! 底层是 `DashMap<TypeId, CompRef>`，运行期解析依赖。
//! 对外提供类型安全的 `inject::<T>()` 接口。

use std::any::{Any, TypeId};
use std::sync::Arc;

use dashmap::DashMap;

use crate::component::Component;
use crate::error::{AppError, DiErr};

/// 存储单元：
/// - `Factory(Arc<dyn Fn>)` → 存工厂闭包，prototype 每次注入时调用
/// - `Cached(Arc<dyn Any>)` → 已实例化的单例（擦除类型）
type FactoryFn = dyn Fn(&Store) -> Arc<dyn Any + Send + Sync> + Send + Sync;

#[derive(Clone)]
pub enum CompRef {
    /// 工厂闭包：Prototype 作用域，每次注入调用
    Factory(Arc<FactoryFn>),
    /// 已缓存的实例：Singleton 作用域
    Cached(Arc<dyn Any + Send + Sync>),
}

/// 组件存储 — 类型安全的注入入口
pub struct Store {
    inner: DashMap<TypeId, CompRef>,
    /// trait 实现的映射表（trait TypeId → 实现列表），由 BuildContext 在构建时填充
    pub(crate) trait_impls: TraitImplMap,
    /// Prototype 实例追踪（TypeId → Weak 引用列表），用于 shutdown 时通知存活实例
    pub(crate) prototype_instances: DashMap<TypeId, Vec<std::sync::Weak<dyn Any + Send + Sync>>>,
}

impl Store {
    /// 创建空 Store
    pub fn new() -> Self {
        Store {
            inner: DashMap::new(),
            trait_impls: DashMap::new(),
            prototype_instances: DashMap::new(),
        }
    }

    /// 从 DashMap 创建 Store（trait_impls 为空，需后续填充）
    pub fn from_dashmap(inner: DashMap<TypeId, CompRef>) -> Self {
        Store {
            inner,
            trait_impls: DashMap::new(),
            prototype_instances: DashMap::new(),
        }
    }

    /// 记录 Prototype 实例（类型擦除版，用于 register_factory）
    ///
    /// 每次工厂创建新实例后调用，将 `Arc` 转为 `Weak` 存储，便于 shutdown 时通知存活实例。
    pub(crate) fn track_prototype_raw(&self, instance: &Arc<dyn Any + Send + Sync>) {
        let tid = (**instance).type_id();
        let weak = Arc::downgrade(instance);
        self.prototype_instances.entry(tid).or_default().push(weak);
    }

    /// 关闭所有存活的 Prototype 实例
    ///
    /// 清理所有已过期的 Weak 条目。
    pub fn shutdown_prototypes(&self) {
        self.prototype_instances.retain(|_tid, weaks| {
            weaks.retain(|weak| {
                // 实例仍存活（强引用未全部释放），保留 Weak；否则移除
                weak.upgrade().is_some()
            });
            !weaks.is_empty()
        });
    }

    /// 获取内部 DashMap 的引用
    pub fn inner(&self) -> &DashMap<TypeId, CompRef> {
        &self.inner
    }

    /// 获取内部 DashMap 的所有权（消耗 self）
    pub fn into_inner(self) -> DashMap<TypeId, CompRef> {
        self.inner
    }

    /// 注册缓存实例（Singleton）
    pub fn insert_cached<T: Any + Send + Sync>(&self, value: T) {
        self.inner
            .insert(TypeId::of::<T>(), CompRef::Cached(Arc::new(value)));
    }

    /// 注册已 Arc 包装的缓存实例
    pub fn insert_arc<T: Any + Send + Sync>(&self, arc: Arc<T>) {
        self.inner.insert(
            TypeId::of::<T>(),
            CompRef::Cached(arc as Arc<dyn Any + Send + Sync>),
        );
    }

    /// 注册工厂闭包（Prototype）
    ///
    /// 每次注入时调用工厂，构造新实例。
    /// `T` 为组件类型，`TypeId` 通过 `TypeId::of::<T>()` 自动获取。
    pub fn insert_factory<T: Any + Send + Sync, F>(&self, factory: F)
    where
        F: Fn(&Store) -> Arc<T> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.inner.insert(
            type_id,
            CompRef::Factory(Arc::new(move |store| {
                factory(store) as Arc<dyn Any + Send + Sync>
            })),
        );
    }

    /// 注入组件实例（类型安全）
    ///
    /// - Singleton：返回缓存的 `Arc<T>`
    /// - Prototype：调用工厂闭包，每次构造新实例
    ///
    /// # Panics
    ///
    /// 组件未注册时 panic（编程错误，不是运行时错误）。
    pub fn inject<T: Component>(&self) -> Result<Arc<T>, AppError> {
        let tid = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // clone 出 CompRef 后释放 DashMap shard 锁，避免 factory 回调中重入死锁
        let comp_ref = self.inner.get(&tid).map(|e| e.clone());
        match comp_ref {
            Some(entry) => {
                let any_arc = match &entry {
                    CompRef::Cached(arc) => arc.clone(),
                    CompRef::Factory(f) => f(self),
                };
                any_arc.downcast::<T>().map_err(|bad_arc| {
                    let actual = (*bad_arc).type_id();
                    AppError::with_context(
                        DiErr::InjectError,
                        format!(
                            "downcast 失败: 期望 `{}`, 实际 TypeId={:?}",
                            type_name, actual
                        ),
                    )
                })
            }
            None => {
                let count = self.inner.len();
                Err(AppError::with_context(
                    DiErr::InjectError,
                    format!(
                        "组件注入失败: `{}` (TypeId={:?}) 未在 Store 中注册。\n\
                         可能原因:\n\
                         1. 组件未标注 #[derive(Component)]\n\
                         2. 所在 crate 或插件未在 Cargo.toml 中引入\n\
                         3. 组件未被使用 (use tx_di_xxx;) — linkme 需此触发注册\n\
                         Store 中已注册 {} 个组件",
                        type_name, tid, count
                    ),
                ))
            }
        }
    }

    /// 注入组件实例（类型安全）— 直接返回 Arc<T>，失败时 panic
    ///
    /// 这是 `inject()` 的便捷版本，用于不需要错误处理的场景。
    pub fn inject_or_panic<T: Component>(&self) -> Arc<T> {
        match self.inject::<T>() {
            Ok(arc) => arc,
            Err(e) => panic!("{}", e),
        }
    }

    /// 尝试注入，失败返回 None
    pub fn try_inject<T: Component>(&self) -> Option<Arc<T>> {
        self.inject::<T>().ok()
    }

    /// 检查组件是否已注册
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.inner.contains_key(&TypeId::of::<T>())
    }

    /// 已注册组件数量
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

// ── 全局注入函数（兼容宏生成的代码）──────────────────────────────────────

/// 从 Store 中注入依赖（类型安全版本）
///
/// 供宏生成的 `build` 方法调用。
/// 从 Store 中尝试按类型注入组件（不 panic 版）
///
/// 组件已注册时返回 `Some(Arc<T>)`，未注册时返回 `None`。
/// 用于 `Option<Arc<T>>` 可选组件注入。
pub fn try_inject_from_store<T: Any + Send + Sync + 'static>(store: &Store) -> Option<Arc<T>> {
    let tid = TypeId::of::<T>();
    let comp_ref = store.inner().get(&tid).map(|e| e.clone());
    match comp_ref {
        Some(entry) => match &entry {
            CompRef::Cached(arc) => arc.clone().downcast::<T>().ok(),
            CompRef::Factory(f) => f(store).downcast::<T>().ok(),
        },
        None => None,
    }
}

/// 通过 Store 注入已注册组件
///
/// # Panics
///
/// 组件未注册时 panic，附带已注册组件列表辅助排查。
pub fn inject_from_store<T: Component>(store: &Store) -> Arc<T> {
    store.inject_or_panic::<T>()
}

// ── Trait Object 注入 ─────────────────────────────────────────────────────

/// trait 实现条目：记录某个 trait 的一个具体实现
#[derive(Clone, Copy)]
pub struct TraitImplEntry {
    /// 具体类型的 TypeId
    pub concrete_tid: fn() -> TypeId,
    /// 将具体实例 (Arc<dyn Any + Send + Sync>) 转型为 trait object
    /// 返回的 Arc<dyn Any + Send + Sync> 内部是 Arc<dyn Trait>
    pub upcast: fn(Arc<dyn Any + Send + Sync>) -> Arc<dyn Any + Send + Sync>,
}

/// trait TypeId → 实现列表的映射表类型
pub type TraitImplMap = DashMap<TypeId, Vec<TraitImplEntry>>;

/// 从 Store 中注入 trait object（返回第一个实现）
///
/// 通过 `store.trait_impls` 查找 trait 的具体实现。
///
/// # Panics
///
/// trait 无实现时 panic。
pub fn inject_trait_from_store<T: ?Sized + Any + Send + Sync + 'static>(store: &Store) -> Arc<T> {
    let tid = TypeId::of::<T>();
    let type_name = std::any::type_name::<T>();

    store
        .trait_impls
        .get(&tid)
        .and_then(|entries| entries.first().cloned())
        .map(|entry| {
            let concrete = store
                .inner()
                .get(&(entry.concrete_tid)())
                .map(|r| match &*r {
                    CompRef::Cached(any_arc) => any_arc.clone(),
                    CompRef::Factory(f) => f(store),
                })
                .unwrap_or_else(|| panic!("[di] trait `{}` 的具体实现未注册到 store", type_name));
            let trait_any = (entry.upcast)(concrete);
            trait_any
                .downcast_ref::<Arc<T>>()
                .expect("[di] trait upcast 类型不匹配")
                .clone()
        })
        .unwrap_or_else(|| {
            panic!(
                "[di] 注入失败: trait `{}` 无任何实现。\n\
                 请确认:\n\
                 1. 实现该 trait 的结构体已标注 #[component(as_trait = dyn Trait)]\n\
                 2. 所在 crate 已在 Cargo.toml 中引入",
                type_name
            )
        })
}

/// 从 Store 中尝试注入 trait object（不 panic 版）
///
/// 若 trait 无实现或实现未就绪，返回 None。用于可选 trait 注入（`Option<Arc<dyn Trait>>`）。
pub fn try_inject_trait_from_store<T: ?Sized + Any + Send + Sync + 'static>(
    store: &Store,
) -> Option<Arc<T>> {
    let tid = TypeId::of::<T>();

    let entry = store.trait_impls.get(&tid)?.first().cloned()?;
    let concrete = store
        .inner()
        .get(&(entry.concrete_tid)())
        .map(|r| match &*r {
            CompRef::Cached(any_arc) => any_arc.clone(),
            CompRef::Factory(f) => f(store),
        })?;
    let trait_any = (entry.upcast)(concrete);
    trait_any.downcast_ref::<Arc<T>>().cloned()
}

/// 从 Store 中注入 trait object 的所有实现（无实现时返回空 Vec）
pub fn inject_all_traits_from_store<T: ?Sized + Any + Send + Sync + 'static>(
    store: &Store,
) -> Vec<Arc<T>> {
    let tid = TypeId::of::<T>();

    store
        .trait_impls
        .get(&tid)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let concrete =
                        store
                            .inner()
                            .get(&(entry.concrete_tid)())
                            .map(|r| match &*r {
                                CompRef::Cached(any_arc) => any_arc.clone(),
                                CompRef::Factory(f) => f(store),
                            })?;
                    let trait_any = (entry.upcast)(concrete);
                    trait_any.downcast_ref::<Arc<T>>().cloned()
                })
                .collect()
        })
        .unwrap_or_default()
}
