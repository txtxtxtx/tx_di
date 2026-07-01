//! 错误类型
//!
//! DI 框架的错误分两类：
//! - **InjectError** — 注入阶段的错误（组件未注册、downcast 失败等）
//! - **RegistryError** — 注册/拓扑排序阶段的错误（循环依赖、依赖缺失等）

use std::any::TypeId;

/// 注入错误
#[derive(Debug)]
pub enum InjectError {
    /// 组件未注册
    NotRegistered {
        /// 期望注入的类型名
        type_name: &'static str,
        /// 期望的 TypeId
        type_id: TypeId,
        /// 已注册组件的 TypeId 列表（用于诊断）
        registered: Vec<TypeId>,
    },

    /// downcast 失败（框架内部 bug）
    DowncastFailed {
        /// 期望的类型名
        expected: &'static str,
        /// 实际的 TypeId
        actual: TypeId,
    },

    /// Trait 无实现
    TraitNotImplemented {
        /// trait 类型名
        trait_name: &'static str,
        /// trait TypeId
        trait_id: TypeId,
    },

    /// Trait 实现未注册到 store
    TraitImplNotInStore {
        /// trait 类型名
        trait_name: &'static str,
        /// 具体实现类型名
        concrete_name: &'static str,
    },
}

impl std::fmt::Display for InjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InjectError::NotRegistered { type_name, type_id, registered } => {
                write!(
                    f,
                    "[di] 注入失败: 组件 `{}` (TypeId={:?}) 未注册。\n\
                     请确认:\n\
                     1. 该结构体已标注 #[derive(Component)]\n\
                     2. 所在 crate 已在 Cargo.toml 中引入\n\
                     已注册组件 ({} 个)",
                    type_name, type_id, registered.len()
                )
            }
            InjectError::DowncastFailed { expected, actual } => {
                write!(f, "[di] 注入 downcast 失败: 期望 `{}`, 实际 TypeId={:?}", expected, actual)
            }
            InjectError::TraitNotImplemented { trait_name, trait_id } => {
                write!(
                    f,
                    "[di] 注入失败: trait `{}` (TypeId={:?}) 无任何实现。\n\
                     请确认:\n\
                     1. 实现该 trait 的结构体已标注 #[component(as_trait = dyn Trait)]\n\
                     2. 所在 crate 已在 Cargo.toml 中引入",
                    trait_name, trait_id
                )
            }
            InjectError::TraitImplNotInStore { trait_name, concrete_name } => {
                write!(f, "[di] trait `{}` 的具体实现 `{}` 未注册到 store", trait_name, concrete_name)
            }
        }
    }
}

impl std::error::Error for InjectError {}

/// 注册表错误
#[derive(Debug)]
pub enum RegistryError {
    /// 循环依赖
    CircularDependency {
        /// 参与循环的组件及其依赖关系
        cycle: Vec<String>,
    },

    /// 依赖未注册
    MissingDependency {
        /// 依赖方的组件名
        component: String,
        /// 缺失的依赖 TypeId
        missing_type_id: TypeId,
        /// 已注册组件列表
        registered: Vec<String>,
    },

    /// 拓扑排序内部错误
    Internal(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::CircularDependency { cycle } => {
                write!(
                    f,
                    "[di] 检测到循环依赖！以下组件形成环路:\n{}\n\
                     请检查这些组件之间是否存在相互依赖，打破环中任意一条边即可。",
                    cycle.join("\n")
                )
            }
            RegistryError::MissingDependency { component, missing_type_id, registered } => {
                write!(
                    f,
                    "[di] 拓扑排序失败: 组件 '{}' 依赖的 TypeId {:?} 未注册。\n\
                     请确认该依赖组件已标注 #[derive(Component)] 且其 crate 已引入。\n\
                     已注册组件 ({} 个): [{}]",
                    component, missing_type_id, registered.len(), registered.join(", ")
                )
            }
            RegistryError::Internal(msg) => {
                write!(f, "[di] 注册表内部错误: {}", msg)
            }
        }
    }
}

impl std::error::Error for RegistryError {}
