//! 组件注册表 — linkme 编译期收集组件元数据
//!
//! `ComponentMeta` 存储核心字段 + 生命周期函数指针。
//! 生命周期函数指针由宏生成，内部调用 `Component` trait 方法。

use std::any::TypeId;
use std::sync::Arc;

use crate::component::BoxFuture;
use crate::scope::Scope;
use crate::store::{Store, TraitImplEntry};
use crate::{RIE, App, CancellationToken};

/// linkme 分布式切片：收集所有 `#[derive(Component)]` 标注的组件
#[linkme::distributed_slice]
pub static COMPONENT_REGISTRY: [ComponentMeta] = [..];

/// 组件注册元数据（linkme 收集用）
///
/// 核心字段 + 生命周期函数指针。
/// 函数指针由宏生成，内部调用 `<T as Component>::xxx()`，
/// 解决 ComponentMeta 类型擦除后无法调用 trait 方法的问题。
pub struct ComponentMeta {
    // ── 核心字段 ──────────────────────────────────────────────────────

    /// 返回组件类型 `TypeId`
    pub type_id: fn() -> TypeId,

    /// 组件类型名（调试用）
    pub name: &'static str,

    /// 依赖类型 ID 列表（用于拓扑排序）
    pub dep_type_ids: &'static [fn() -> TypeId],

    /// 工厂函数：从 Store 构建组件，返回类型擦除的 Box
    pub factory: fn(&Store) -> Box<dyn std::any::Any + Send + Sync>,

    /// 作用域
    pub scope: Scope,

    /// 该组件实现的 trait TypeId 列表
    pub impl_traits: &'static [fn() -> TypeId],

    /// trait 实现条目列表（用于填充 TRAIT_IMPL_MAP）
    pub trait_impls: &'static [TraitImplEntry],

    // ── 生命周期函数指针（宏生成，内部调用 Component trait 方法）──────

    /// 初始化排序值（值越小越先执行）
    pub init_sort_fn: fn() -> i32,

    /// 同步初始化（App 阶段）
    pub init_fn: fn(&Arc<App>) -> RIE<()>,

    /// 异步初始化（App 阶段）
    pub async_init_fn: fn(&Arc<App>) -> BoxFuture<RIE<()>>,

    /// 异步运行（后台 task）
    pub async_run_fn: fn(&Arc<App>, CancellationToken) -> BoxFuture<RIE<()>>,

    /// 优雅关闭
    pub shutdown_fn: fn(&Store),
}

impl ComponentMeta {
    /// 调用工厂函数构建组件实例
    pub fn build(&self, store: &Store) -> Box<dyn std::any::Any + Send + Sync> {
        (self.factory)(store)
    }

    /// 获取组件类型 ID
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// 获取依赖类型 ID 列表
    pub fn dep_ids(&self) -> Vec<TypeId> {
        self.dep_type_ids.iter().map(|f| f()).collect()
    }
}
