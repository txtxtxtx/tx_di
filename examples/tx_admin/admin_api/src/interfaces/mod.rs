//! 接口层：HTTP (axum)，通过 DiComp<T> 从 DI 容器注入 AppService

pub mod api;   // HTTP 处理器
pub mod dto;   // 通用 DTO / 响应包装
