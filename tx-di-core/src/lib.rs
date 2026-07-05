//! # tx-di-core
//!
//! 类型驱动的 Rust 依赖注入框架。
//!
//! ## 核心概念
//!
//! - **Component trait** — 每个被 DI 管理的类型实现此 trait，用 associated type 声明依赖
//! - **ComponentMeta** — 瘦注册条目，linkme 编译期收集，运行期拓扑排序
//! - **Store** — 类型擦除的组件存储（DashMap<TypeId, CompRef>），运行期解析依赖
//! - **AOP** — Interceptor trait + proc_macro 代理，零运行时开销
//!
//! ## 设计原则
//!
//! 1. 类型驱动：依赖在 `type Deps` 中声明，编译期可知
//! 2. 编译期收集：linkme 零开销注册
//! 3. 运行期解析：拓扑排序 + DashMap 存储
//! 4. 可扩展：ComponentMeta 只存核心字段，生命周期钩子在 trait 默认方法中

pub mod aop;
pub mod component;
pub mod config;
pub mod error;
pub mod lifecycle;
pub mod registry;
pub mod scope;
pub mod store;
pub mod topology;

// ── 第三方 re-export ──────────────────────────────────────────────────────
pub use dashmap;
pub use dashmap::DashMap;
pub use linkme;
pub use toml;
pub use toml::Value;
pub use toml::map;

// ── 内部模块 re-export ────────────────────────────────────────────────────
// 注意：derive 宏 `Component` 和 trait `Component` 同名但不同命名空间，可以共存
// `tx_cst` 和 `component` 是 derive 辅助属性，不需要单独 re-export
pub use tx_di_macros::Component;   // derive 宏（宏命名空间）
pub use tx_error::{AppErrCode, AppError, AppResult, CodeMsg};
pub use crate::error::DiErr;
pub use tx_common::{ApiR, ApiRes, FormattedDateTime, RCode};

/// RIE<T> = AppResult<T>
pub type RIE<T> = AppResult<T>;

pub use tokio_util::sync::CancellationToken;

// ── 核心 re-export ────────────────────────────────────────────────────────
pub use component::{BoxFuture, Component, DepsTuple};
pub use config::AppAllConfig;
// 内部错误模块：直接复用 tx_error 提供的统一错误类型
// 详见 src/error.rs
pub use lifecycle::{App, BuildContext, InnerContext, get_sys_config, set_sys_config, CONFIG_PATH};
pub use registry::{ComponentMeta, COMPONENT_REGISTRY};
pub use scope::Scope;
pub use store::{Store, CompRef, TraitImplEntry, TraitImplMap, inject_from_store, inject_trait_from_store, inject_all_traits_from_store};
pub use topology::topo_sort;
pub use aop::{CallContext, CallResult, Interceptor, InterceptorChain};

/// 简化异步方法实现的宏
///
/// 将用户写的 `fn name(...) -> RIE<()> { ... }` 转换为
/// `fn name(...) -> BoxFuture<RIE<()>> { Box::pin(async move { ... }) }`，
/// 自动处理 `BoxFuture` 包装，用户无需手动写 `Box::pin`。
#[macro_export]
macro_rules! async_method {
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident($($param:ident: $ty:ty),* $(,)?) -> $ret:ty $body:block
    ) => {
        $(#[$meta])*
        $vis fn $name($($param: $ty),*) -> $crate::BoxFuture<$ret> {
            Box::pin(async move $body)
        }
    };
}
