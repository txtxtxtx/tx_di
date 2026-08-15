//! 密码安全哈希模块（兼容层）
//!
//! 实际实现已迁移至 [`crate::shared::security::password`]，
//! 此模块保留 `crate::password` 路径以兼容既有调用方。

pub use crate::shared::security::password::*;
