//! 接口层：同时支持 HTTP (axum) 和 gRPC (tonic)
//!
//! 两种协议共用 admin_proto 生成的传输对象。

pub mod api;   // HTTP 处理器
pub mod dto;   // 通用 DTO / 响应包装
pub mod grpc;  // gRPC 服务实现
