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
#[derive(Clone)]
pub enum CompRef {
    /// 工厂闭包：Prototype 作用域，每次注入调用
    Factory(Arc<dyn Fn(&Store) -> Arc<dyn Any + Send + Sync> + Send + Sync>),
    /// 已缓存的实例：Singleton 作用域
    Cached(Arc<dyn Any + Send + Sync>),
}

/// 组件存储 — 类型安全的注入入口
pub struct Store {
    inner: DashMap<TypeId, CompRef>,
}

impl Store {
    /// 创建空 Store
    pub fn new() -> Self {
        Store {
            inner: DashMap::new(),
        }
    }

    /// 从 DashMap 创建 Store
    pub fn from_dashmap(inner: DashMap<TypeId, CompRef>) -> Self {
        Store { inner }
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
        self.inner
            .insert(TypeId::of::<T>(), CompRef::Cached(arc as Arc<dyn Any + Send + Sync>));
    }

    /// 注册工厂闭包（Prototype）
    pub fn insert_factory<F>(&self, factory: F)
    where
        F: Fn(&Store) -> Arc<dyn Any + Send + Sync> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<Arc<dyn Any + Send + Sync>>(); // placeholder
        // 注意：这个方法需要调用方提供 TypeId
        // 实际使用中，工厂注册由 registry 自动完成
        let _ = type_id;
        // todo 此方法保留给未来扩展
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

        match self.inner.get(&tid) {
            Some(entry) => {
                let any_arc = match &*entry {
                    CompRef::Cached(arc) => arc.clone(),
                    CompRef::Factory(f) => f(self),
                };
                any_arc.downcast::<T>().map_err(|bad_arc| {
                    let actual = (&*bad_arc).type_id();
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
                let registered: Vec<TypeId> = self.inner.iter().map(|e| *e.key()).collect();
                Err(AppError::with_context(
                    DiErr::InjectError,
                    format!(
                        "组件 `{}` (TypeId={:?}) 未注册。\n\
                         请确认:\n\
                         1. 该结构体已标注 #[derive(Component)]\n\
                         2. 所在 crate 已在 Cargo.toml 中引入\n\
                         已注册组件 ({} 个): {:?}",
                        type_name, tid, registered.len(), registered
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

/// 全局 trait 实现映射表，在 `auto_register_all` 阶段填充
pub static TRAIT_IMPL_MAP: std::sync::LazyLock<TraitImplMap> =
    std::sync::LazyLock::new(DashMap::new);

/// 从 Store 中注入 trait object（返回第一个实现）
///
/// 通过 `TRAIT_IMPL_MAP` 查找 trait 的具体实现。
///
/// # Panics
///
/// trait 无实现时 panic。
pub fn inject_trait_from_store<T: ?Sized + Any + Send + Sync + 'static>(
    store: &Store,
) -> Arc<T> {
    let tid = TypeId::of::<T>();
    let type_name = std::any::type_name::<T>();

    TRAIT_IMPL_MAP
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
                .unwrap_or_else(|| {
                    panic!(
                        "[di] trait `{}` 的具体实现未注册到 store",
                        type_name
                    )
                });
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

/// 从 Store 中注入 trait object 的所有实现
pub fn inject_all_traits_from_store<T: ?Sized + Any + Send + Sync + 'static>(
    store: &Store,
) -> Vec<Arc<T>> {
    let tid = TypeId::of::<T>();

    TRAIT_IMPL_MAP
        .get(&tid)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    let concrete = store
                        .inner()
                        .get(&(entry.concrete_tid)())
                        .map(|r| match &*r {
                            CompRef::Cached(any_arc) => any_arc.clone(),
                            CompRef::Factory(f) => f(store),
                        })
                        .expect("[di] trait 具体实现未注册到 store");
                    let trait_any = (entry.upcast)(concrete);
                    trait_any
                        .downcast_ref::<Arc<T>>()
                        .expect("[di] trait upcast 类型不匹配")
                        .clone()
                })
                .collect()
        })
        .unwrap_or_default()
}
