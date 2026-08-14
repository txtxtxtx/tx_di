//! # admin_api 库入口
//!
//! bin（`src/main.rs`）与集成测试（`tests/`）共享本库：
//! - 组件模块在此统一声明（bin 不再重复声明，避免组件重复注册到 `COMPONENT_REGISTRY`）
//! - 空 `use` 显式引用各插件 crate，触发 linkme 编译期注册
//!   （插件 crate 被链接器优化掉会导致组件静默失效）
//!
//! ## 集成测试约定
//! 测试文件通过 `use admin_api;`（空导入）引用本库，使 `AdminPlugin` 等组件的
//! linkme 注册条目进入测试二进制，从而用 `BuildContext::with_config` 启动完整应用。

pub mod auth;
pub mod error;
pub mod interfaces;
pub mod nacos;
pub mod operate_log;
pub mod plugin;

// 显式引用各插件 crate，触发 linkme 编译期注册（组件注册点）
#[allow(unused_imports, clippy::single_component_path_imports)]
use {
    admin_app, admin_infra, tx_di_axum, tx_di_file, tx_di_job, tx_di_log, tx_di_nacos,
    tx_di_sa_token, tx_di_toasty,
};
