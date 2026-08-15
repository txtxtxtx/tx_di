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
pub mod config_env;
pub mod error;
pub mod lifecycle;
pub mod path_utils;
pub mod registry;
pub mod scope;
pub mod store;
pub mod topology;

// ── 第三方 re-export ──────────────────────────────────────────────────────
pub use dashmap::DashMap;
pub use linkme;
pub use serde;
pub use serde_json;
pub use toml::Value;
pub use toml::map;
pub use tracing;

// ── 内部模块 re-export ────────────────────────────────────────────────────
// 注意：derive 宏 `Component` 和 trait `Component` 同名但不同命名空间，可以共存
// `tx_cst` 和 `component` 是 derive 辅助属性，不需要单独 re-export
pub use crate::error::DiErr;
pub use tx_common::{ApiR, ApiRes, FormattedDateTime, RCode};
pub use tx_di_macros::Component; // derive 宏（宏命名空间）
pub use tx_di_macros::intercept; // AOP 方法拦截属性宏（#[intercept]）
pub use tx_error::{AppErrCode, AppError, AppResult, CodeMsg};

/// RIE<T> = AppResult<T>
pub type RIE<T> = AppResult<T>;

pub use tokio_util::sync::CancellationToken;

// ── 核心 re-export ────────────────────────────────────────────────────────
pub use component::{BoxFuture, Component, DepsTuple};
pub use config::AppAllConfig;
pub use config_env::{ensure_dotenv, interpolate_env};
pub use path_utils::{resolve_data_path, resolve_sqlite_url};
// 内部错误模块：直接复用 tx_error 提供的统一错误类型
// 详见 src/error.rs
pub use aop::{ArgValue, BoxCall, CallContext, CallFn, CallResult, Interceptor, InterceptorChain};
pub use lifecycle::{App, BuildContext, InnerContext};
pub use registry::{COMPONENT_REGISTRY, ComponentMeta};
pub use scope::Scope;
pub use store::{
    CompRef, Store, TraitImplEntry, TraitImplMap, inject_all_traits_from_store, inject_from_store,
    inject_trait_from_store, try_inject_from_store, try_inject_trait_from_store,
};
pub use topology::topo_sort;
