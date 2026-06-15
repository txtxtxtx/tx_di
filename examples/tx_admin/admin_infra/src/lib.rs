//! 基础设施层 - Toasty ORM Repository 实现
//!
//! 实现 domain 层定义的所有 Repository trait，
//! 使用 toasty ORM 连接数据库（SQLite/PostgreSQL/MySQL）。

pub mod user;
pub mod role;
pub mod permission;
pub mod menu;
pub mod department;
pub mod file;
pub mod config;
pub mod dictionary;
pub mod log;
pub mod plugin;
pub mod seed;

/// 注册所有 toasty 数据库模型
///
/// 自动扫描本 crate 中所有 `#[derive(Model)]` 的类型，
/// 传给 `ToastyPlugin::register_models()` 使用。
///
/// # 用法
/// ```ignore
/// let plugin = ctx.inject::<ToastyPlugin>();
/// plugin.register_models(admin_infra::register_models());
/// ```
pub fn register_models() -> toasty::ModelSet {
    toasty::models!(crate::*)
}
