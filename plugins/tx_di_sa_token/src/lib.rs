//! tx_di_sa_token — 基于 sa-token-rust 的 tx-di 认证授权插件
//!
//! 封装 [sa-token-rust](https://github.com/sa-tokens/sa-token-rust) 认证授权框架，
//! 集成到 tx-di 依赖注入框架，提供：
//! - 登录认证（JWT / Session）
//! - 权限和角色控制
//! - 分布式 Session（内存 / Redis / 数据库）
//! - SSO 单点登录
//! - OAuth2.0
//!
//! # 快速开始
//!
//! ```toml
//! # Cargo.toml
//! tx_di_sa_token = { path = "plugins/tx_di_sa_token", features = ["memory"] }
//! ```
//!
//! ```toml
//! # config/config.toml
//! [sa_token_config]
//! token_name = "Authorization"
//! timeout = 86400
//! is_concurrent = true
//! is_share = true
//! token_style = "uuid"
//! ```
//!
//! # Feature Flags
//!
//! | Feature    | 存储后端     |
//! |-----------|-------------|
//! | `memory`  | 内存（默认）  |
//! | `redis`   | Redis       |
//! | `database` | 数据库      |

mod config;
mod plugin;

pub use config::SaTokenConf;
pub use plugin::SaTokenPlugin;

// 重导出 sa-token 核心 API
pub use sa_token_plugin_axum::{
    SaTokenLayer, SaCheckLoginLayer, SaCheckPermissionLayer,
    SaTokenExtractor, OptionalSaTokenExtractor, LoginIdExtractor,
    SaTokenState, SaTokenStateBuilder,
    StpUtil,
    sa_check_login, sa_check_permission, sa_check_role,
    sa_check_permissions_and, sa_check_permissions_or,
    sa_check_roles_and, sa_check_roles_or,
    sa_ignore,
    SaStorage, MemoryStorage,
};
