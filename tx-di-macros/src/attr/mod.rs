//! 属性解析模块
//!
//! 负责解析 derive 宏的辅助属性：
//! - `#[component(...)]` — 结构体属性
//! - `#[tx_cst(...)]` — 字段属性

pub mod comp_attr;
pub mod field_attr;
