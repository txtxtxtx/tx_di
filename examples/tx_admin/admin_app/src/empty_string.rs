//! 空字符串转 None 辅助模块
//!
//! 前端发送 `""` 时，转为 `None`。
//! 用于 Proto 请求 → App Command 转换时处理空字符串。

/// 将空字符串转为 `None`
///
/// ```ignore
/// use admin_app::empty_string::opt;
///
/// let s = opt(req.remark);  // "" → None, "hello" → Some("hello")
/// ```
pub fn opt(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// 将 Option 中的空字符串转为 None
pub fn opt_filter(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}
