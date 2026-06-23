//! gRPC 服务实现
//!
//! 各模块的 tonic service trait 实现都在此目录下。
//! 所有 gRPC 服务使用 admin_proto 生成的 DTO，与 HTTP 共用。

pub mod auth_interceptor;
pub mod err;

pub mod auth_service;
pub mod user_service;
pub mod role_service;
pub mod menu_service;
pub mod dept_service;
pub mod config_service;
pub mod dict_service;
pub mod log_service;
pub mod file_service;
pub mod monitor_service;
pub mod tool_service;
pub mod job_service;
