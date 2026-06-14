//! 空字符串转 None 的 serde 辅助模块
//!
//! 前端发送 `""` 时，自动反序列化为 `None`。
//!
//! # 用法
//! ```ignore
//! #[derive(Deserialize)]
//! struct MyDto {
//!     #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string")]
//!     pub remark: Option<String>,
//! }
//! ```

use serde::{Deserialize, Deserializer};

/// 将空字符串 `""` 反序列化为 `None`，非空字符串为 `Some(s)`
pub fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}
