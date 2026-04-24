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
pub use toml;
pub use toml::Value;
pub use toml::map;

pub use tx_di_macros::{tx_comp, tx_cst};
pub use di::{BuildContext,scopes::Scope,App,
             comp::{ComponentMeta,topo_sort,COMPONENT_REGISTRY,config::AppAllConfig},
             comp::comp_ref::{CompRef,ComponentDescriptor,CompInit,BoxFuture},
common::*
};

