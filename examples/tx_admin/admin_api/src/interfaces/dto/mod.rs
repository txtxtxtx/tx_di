//! HTTP 协议通用 DTO / 响应包装
//!
//! 统一使用 tx_common::{ApiR, ApiRes, Page} 作为响应类型。
//! ApiR<T> 直接实现 IntoResponse，AppError 也直接实现 IntoResponse。
//! 本模块保留为未来可能的 HTTP 层专用 DTO 扩展点。

// 复用 tx_common 中的通用响应类型
// pub use tx_common::api_r::{ApiR, ApiRes};
// pub use tx_common::page::Page;