//! admin_server — DDD 后台管理系统
//!
//! 采用领域驱动设计（DDD）架构，提供：
//! - **RBAC 动态权限**：用户 → 角色 → 权限，支持按钮级/API级权限控制
//! - **数据权限**：全部/部门及子部门/本部门/仅本人/自定义五种数据范围
//! - **SaaS 多租户**：租户隔离、租户套餐、租户状态管理
//! - **文件服务**：统一文件存储抽象，支持本地文件系统和 S3/MinIO
//!
//! # 架构分层
//!
//! ```text
//! interfaces/     ← HTTP API 路由、请求/响应 DTO
//! application/    ← 应用服务（用例编排）
//! domain/         ← 领域实体、仓储 trait、领域服务
//! infrastructure/ ← 仓储实现、缓存、外部集成
//! ```
//!
//! 的设计思想，使用 Rust + DDD 重新实现核心能力。

/// 领域层：实体、值对象、仓储 trait、领域服务
pub mod domain;

/// 应用层：用例编排、DTO 转换、事务管理
pub mod application;

/// 基础设施层：仓储实现、缓存、外部服务适配
pub mod infrastructure;

/// 接口层：HTTP API 路由、请求/响应 DTO
pub mod interfaces;

/// 后台管理插件（路由注册）
pub mod admin_plugin;
