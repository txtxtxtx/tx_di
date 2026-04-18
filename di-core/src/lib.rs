//! # di-core
//!
//! 编译期依赖注入框架的核心运行时支撑层。
//!
//! ## 核心概念
//!
//! ### Scope（作用域）
//! - **Singleton**：全局共享，工厂调用一次，缓存 `Arc<T>`
//! - **Prototype**：每次注入调用工厂，构造新实例
//!
//! ### 设计原则
//! - **scope 标记在被注入的组件上**，消费者字段写裸类型
//! - 字段类型 `T` 或 `Arc<T>` 均可，框架通过 `TypeId` 找到对应的工厂
//! - 统一通过 `ctx.inject::<T>()` 注入，自动根据组件自身 scope 决定行为

use std::any::{Any, TypeId};
use std::sync::Arc;

pub use linkme;

use dashmap::DashMap;
use log::debug;
// ─────────────────────────────────────────────────────────────────────────────
// 1. Scope
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Singleton,
    Prototype,
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. CompRef —— 统一存储工厂函数
// ─────────────────────────────────────────────────────────────────────────────

/// 存储单元：
/// - `Factory(Arc<dyn Fn>)` → 存工厂闭包，prototype 每次注入时调用
/// - `Cached(Arc<dyn Any>)` → 已实例化的单例（擦除类型）
pub enum CompRef {
    /// 尚未实例化的工厂（prototype），直接存闭包
    Factory(Arc<dyn Fn(&mut BuildContext) -> Arc<dyn Any + Send + Sync> + Send + Sync>),
    /// 已实例化并缓存的单例
    Cached(Arc<dyn Any + Send + Sync>),
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. 全局组件注册表
// ─────────────────────────────────────────────────────────────────────────────

pub struct ComponentMeta {
    pub type_id: fn() -> TypeId,
    pub deps: &'static [fn() -> TypeId],
    pub name: &'static str,
    pub scope: Scope,
    /// 原始工厂函数（用于 `debug_registry` 诊断；运行时不使用）
    pub factory_fn: Option<fn(&mut BuildContext) -> Box<dyn Any + Send + Sync>>,
}

#[linkme::distributed_slice]
pub static COMPONENT_REGISTRY: [ComponentMeta] = [..];

// ─────────────────────────────────────────────────────────────────────────────
// 4. BuildContext
// ─────────────────────────────────────────────────────────────────────────────

pub struct BuildContext {
    /// TypeId → CompRef（使用 DashMap 支持并发访问）
    store: DashMap<TypeId, CompRef>,
}

impl BuildContext {
    // #[inline]
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
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
        factory: fn(&mut BuildContext) -> Box<T>,
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
                let closure = move |ctx: &mut BuildContext| -> Arc<dyn Any + Send + Sync> {
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
        factory: fn(&mut BuildContext) -> Box<dyn Any + Send + Sync>,
    ) {
        match scope {
            Scope::Singleton => {
                let instance: Box<dyn Any + Send + Sync> = factory(self);
                let arc: Arc<dyn Any + Send + Sync> = Arc::from(instance);
                self.store.insert(type_id, CompRef::Cached(arc));
            }
            Scope::Prototype => {
                let factory_fn = factory;
                let closure = move |ctx: &mut BuildContext| -> Arc<dyn Any + Send + Sync> {
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
                    "[di] inject::<{}> 未找到，请确认 app!{{}} 中包含该组件",
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

    // ── 兼容旧 API ───────────────────────────────────────────────────────────

    /// 取出单例的 Arc。
    pub fn get_arc<T: Any + Send + Sync + 'static + ComponentDescriptor>(&mut self) -> Arc<T> {
        self.inject::<T>()
    }

    /// 从上下文中取出并移除单例（所有权）。
    pub fn take<T: Any + Send + Sync + 'static>(&mut self) -> T {
        let entry = self
            .store
            .remove(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("[di] take::<{}> 未找到", std::any::type_name::<T>()))
            .1;

        match entry {
            CompRef::Cached(any_arc) => {
                let arc_t: Arc<T> = any_arc.downcast::<T>().unwrap_or_else(|_| {
                    panic!("[di] take downcast 失败：{}", std::any::type_name::<T>())
                });
                Arc::try_unwrap(arc_t).unwrap_or_else(|_| {
                    panic!(
                        "[di] take::<{}> 失败：仍有其他强引用（Arc 计数 > 1）",
                        std::any::type_name::<T>()
                    )
                })
            }
            _ => {
                panic!(
                    "[di] take::<{}> 只能用于单例组件",
                    std::any::type_name::<T>()
                )
            }
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
    pub fn debug_registry() {
        for meta in COMPONENT_REGISTRY.iter() {
            println!(
                "  {:20} scope={:?}  deps={}",
                meta.name,
                meta.scope,
                meta.deps.len()
            );
        }
    }
}

impl Default for BuildContext {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. ComponentDescriptor trait
// ─────────────────────────────────────────────────────────────────────────────

pub trait ComponentDescriptor: Any + Sized + Send + Sync + 'static {
    const DEP_IDS: &'static [fn() -> TypeId];
    const SCOPE: Scope;
    fn build(ctx: &mut BuildContext) -> Self;
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. 拓扑排序（debug 辅助）
// ─────────────────────────────────────────────────────────────────────────────

pub fn topo_sort(metas: &[&ComponentMeta]) -> Vec<TypeId> {
    use std::collections::{HashMap, VecDeque};

    let n = metas.len();

    let id_to_name: HashMap<TypeId, &str> = metas.iter().map(|m| ((m.type_id)(), m.name)).collect();

    let id_to_idx: HashMap<TypeId, usize> = metas
        .iter()
        .enumerate()
        .map(|(i, m)| ((m.type_id)(), i))
        .collect();

    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];

    for (i, meta) in metas.iter().enumerate() {
        for dep_fn in meta.deps {
            let one_type_id = dep_fn();
            if let Some(&j) = id_to_idx.get(&one_type_id) {
                adj[j].push(i);
                in_degree[i] += 1;
            } else {
                panic!(
                    "[di] 组件 '{}' 依赖的类型 {:?} {:?} 未在注册表中找到",
                    meta.name,
                    id_to_name.get(&one_type_id),
                    &one_type_id
                );
            }
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut result = Vec::with_capacity(n);

    while let Some(i) = queue.pop_front() {
        result.push((metas[i].type_id)());
        for &j in &adj[i] {
            in_degree[j] -= 1;
            if in_degree[j] == 0 {
                queue.push_back(j);
            }
        }
    }

    if result.len() != n {
        let cycles: Vec<&str> = metas
            .iter()
            .enumerate()
            .filter(|(i, _)| in_degree[*i] > 0)
            .map(|(_, m)| m.name)
            .collect();
        panic!("[di] 循环依赖：{:?}", cycles);
    }

    let sorted_names: Vec<&str> = result
        .iter()
        .map(|t| {
            id_to_name.get(t).copied().unwrap_or_else(|| {
                panic!("[di] 拓扑排序内部错误：TypeId {:?} 未在名称映射中找到", t)
            })
        })
        .collect();
    debug!("[di] 拓扑排序结果：\n[\n{}\n]", sorted_names.join(",\n"));

    result
}
