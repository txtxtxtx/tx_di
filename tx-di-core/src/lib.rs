//! # tx-di-core
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

mod di;

pub use linkme;
pub use dashmap;
pub use dashmap::DashMap;
pub use toml;
pub use toml::Value;
pub use toml::map;

pub use tx_di_macros::{tx_comp, tx_cst};
pub use tx_error::{CodeMsg, AppErrCode, AppError, AppResult, DiErr};
/// 兼容旧代码：RIE<T> = AppResult<T>
pub type RIE<T> = AppResult<T>;
/// 兼容旧代码：IE = AppError
pub type IE = AppError;
pub use tx_common::{ApiR, ApiRes, RCode, FormattedDateTime};
pub use di::{BuildContext, scopes::Scope, App, InnerContext,
             comp::{ComponentMeta, topo_sort, COMPONENT_REGISTRY, config::AppAllConfig},
             comp::comp_ref::{CompRef, ComponentDescriptor, CompInit, BoxFuture, inject_from_store, inject_trait_from_store, TraitWrapper},
};
pub use tokio_util::sync::CancellationToken;

/// 查找实现了指定 trait 的具体类型名称。
///
/// 该函数在编译期被宏调用，用于查找实现了特定 trait 的组件类型名称。
/// 返回的名称可以用于在 COMPONENT_REGISTRY 中定位具体的组件。
///
/// # 参数
///
/// - `trait_name`: trait 的名称（不含 `dyn` 前缀）
///
/// # 返回值
///
/// 返回实现了该 trait 的组件类型名称，如果未找到则返回 `None`。
///
/// # 示例
///
/// ```ignore
/// // 假设有以下定义：
/// #[tx_comp(as_trait = "UserRepository")]
/// pub struct SqliteUserRepository { ... }
///
/// // 调用：
/// let name = find_impl_type_for_trait("UserRepository");
/// assert_eq!(name, Some("SqliteUserRepository"));
/// ```
pub fn find_impl_type_for_trait(trait_name: &str) -> Option<&'static str> {
    COMPONENT_REGISTRY.iter().find_map(|meta| {
        if meta.impl_traits.contains(&trait_name) {
            Some(meta.name)
        } else {
            None
        }
    })
}
