//! `#[tx_cst(...)]` 字段属性解析
//!
//! 支持两种写法：
//! - `#[tx_cst(expr)]` — 用表达式赋值该字段
//! - `#[tx_cst(skip)]` — 跳过注入，使用 `Default::default()`

use syn::{Attribute, Expr, Ident, Result as SynResult};

/// derive 辅助属性名
pub const TX_CST: &str = "tx_cst";

/// 从属性列表中提取注入表达式
///
/// 遍历属性列表查找标识为 `tx_cst` 的属性，若找到则解析其参数为表达式返回。
///
/// # 返回值
/// - `Ok(Some(expr))` — 找到 `#[tx_cst(expr)]`
/// - `Ok(None)` — 未找到 `tx_cst` 属性
/// - `Err(_)` — 解析失败
pub fn extract_inject_expr(attrs: &[Attribute]) -> SynResult<Option<Expr>> {
    for attr in attrs {
        if attr.path().is_ident(TX_CST) {
            let expr: Expr = attr.parse_args()?;
            return Ok(Some(expr));
        }
    }
    Ok(None)
}

/// 检查属性列表中是否包含 `#[tx_cst(skip)]` 属性
pub fn has_skip_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident(TX_CST)
            && matches!(attr.parse_args::<Ident>(), Ok(ident) if ident == "skip")
    })
}
