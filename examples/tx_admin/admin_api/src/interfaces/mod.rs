//! 接口层：HTTP (axum) 和 gRPC (tonic)，通过 DiComp<T> 从 DI 容器注入 AppService

pub mod api;   // HTTP 处理器
pub mod grpc;  // gRPC 服务实现
