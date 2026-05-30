//! admin_server — DDD 后台管理系统
//!
//! ```text
//! domain/         ← 领域实体 + 仓储 trait + toasty 实现（每个聚合一个子目录）
//! application/    ← 应用服务（用例编排）
//! interfaces/     ← HTTP API + DTO
//! ```

pub mod domain;
pub mod interfaces;
pub mod admin_plugin;
