//! tx_di_job — 基于 tx_di 框架的定时任务调度插件
//!
//! 提供 Cron 表达式调度、多种任务执行器（内部函数、Shell 脚本、Python 脚本）、
//! 任务执行日志记录、任务重试机制和超时监控等功能。
//!
//! # 功能特性
//!
//! - **Cron 表达式调度**：支持标准的 Cron 表达式，精确到秒
//! - **多种任务执行器**：
//!   - 内部函数执行器：执行 Rust 异步函数
//!   - Shell 脚本执行器：执行 Shell 脚本
//!   - Python 脚本执行器：执行 Python 脚本
//! - **任务重试机制**：支持配置重试次数和重试间隔
//! - **超时监控**：支持配置任务执行超时时间
//! - **执行日志记录**：记录每次任务执行的详细信息
//! - **任务管理 API**：提供创建、更新、删除、暂停、恢复等 API
//!
//! # 快速开始
//!
//! ## 1. 添加依赖
//!
//! ```toml
//! # Cargo.toml
//! tx_di_job = { path = "plugins/tx_di_job" }
//! ```
//!
//! ## 2. 配置
//!
//! 在 `configs/di-config.toml` 中添加配置：
//!
//! ```toml
//! [job_config]
//! enabled = true
//! poll_interval_secs = 1
//! shell_timeout_secs = 300
//! python_timeout_secs = 300
//! python_path = "/usr/bin/python3"
//! thread_pool_size = 4
//! ```
//!
//! ## 3. 注册模型
//!
//! 在 `main.rs` 中，在 `build()` 之前注册模型：
//!
//! ```rust,ignore
//! use tx_di_toasty::ToastyPlugin;
//! use tx_di_job::models;
//!
//! // 注册 Job 插件模型
//! ToastyPlugin::register_models(models::register_models());
//!
//! let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;
//! ```
//!
//! ## 4. 注册任务处理器
//!
//! ```rust,ignore
//! let app = BuildContext::new(Some("config.toml")).build()?.ins_run().await?;
//!
//! // 注册内部任务处理器
//! let job_plugin = app.inject::<JobPlugin>();
//! job_plugin.register_handler("cleanup_logs", |param| {
//!     tracing::info!("清理日志开始");
//!     // 清理逻辑
//!     JobResult {
//!         status: ExecutionStatus::Success,
//!         result: Some("清理了 100 条日志".to_string()),
//!         error: None,
//!     }
//! });
//! ```
//!
//! ## 5. 创建任务
//!
//! 通过 API 创建任务：
//!
//! ```json
//! POST /api/jobs
//! {
//!     "name": "清理日志",
//!     "handler_name": "cleanup_logs",
//!     "cron_expression": "0 0 2 * * ?",
//!     "retry_count": 3,
//!     "retry_interval": 60
//! }
//! ```
//!
//! # 任务类型
//!
//! ## 内部函数任务
//!
//! `handler_name` 存储函数名称，如 `"send_email"`。
//!
//! ## Shell 脚本任务
//!
//! `handler_name` 存储脚本路径，如 `"/opt/scripts/backup.sh"`。
//!
//! ## Python 脚本任务
//!
//! `handler_name` 存储脚本路径，如 `"/opt/scripts/analyze.py"`。
//!
//! # 数据库表
//!
//! 插件使用以下数据库表：
//!
//! - `infrust_job` - 定时任务表
//! - `infrust_job_log` - 定时任务日志表
//!
//! 表结构会自动创建（通过 Toasty 的 `auto_schema` 功能）。
//!
//! # Feature Flags
//!
//! 当前版本暂不支持 Feature Flags。

// 私有模块
mod config;
mod err;
mod models;
mod repository;
mod comp;

// 公共模块
pub mod executors;

// 重导出
pub use config::JobConfig;
pub use err::JobErr;
pub use err::JobResult;

pub use models::{InfrustJob, InfrustJobLog, AuditFields, SoftDelete, register_models, ExecutionStatus, JobStatus};
pub use repository::JobRepository;
pub use comp::JobPlugin;
pub use executors::{JobExecutor, ExecutorType, InternalJobExecutor, ShellJobExecutor, PythonJobExecutor};
