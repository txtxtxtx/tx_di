pub mod config;
pub mod handlers;
pub mod manager;

pub use config::Gb28181Config;
pub use manager::Gb28181Manager;

/// 在 DI 框架启动前注册所有 SIP 消息处理器
///
/// 必须在 `BuildContext::build()` 之前调用，以确保服务启动后立即能处理消息。
pub fn register_handlers() {
    handlers::register_all();
}
