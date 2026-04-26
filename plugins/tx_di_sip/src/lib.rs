//! # tx_di_sip
//!
//! 基于 [rsipstack](https://crates.io/crates/rsipstack) 的 SIP 服务插件，
//! 集成到 tx-di 依赖注入框架中，提供开箱即用的 SIP 协议能力。
//!
//! ## 功能概览
//!
//! - **IPv4 / IPv6 双栈支持** — 通过配置 `host` 字段选择监听地址
//! - **UDP + TCP 双传输层** — 可单独或同时启用
//! - **消息处理注册** — 类似 axum 路由的 [`SipRouter::add_handler`] 机制
//! - **消息发送接口** — [`SipSender`] 提供 `register()`/`invite()` 等便捷 API
//! - **优雅停止** — 通过 `CancellationToken` 支持 shutdown
//!
//! ## 快速开始
//!
//! ### 1. 添加依赖
//!
//! 在 `Cargo.toml` 中：
//! ```toml
//! [dependencies]
//! tx_di_sip = { path = "plugins/tx_di_sip" }
//! ```
//!
//! ### 2. 配置文件（`di-config.toml`）
//!
//! ```toml
//! [sip_config]
//! host     = "0.0.0.0"    # 监听地址（IPv4）
//! port     = 5060          # SIP 端口
//! transport = "udp"        # 传输协议: udp / tcp / both
//! user_agent = "MyApp/1.0"
//! ```
//!
//! ### 3. 注册消息处理器 & 启动
//!
//! ```rust,no_run
//! use tx_di_sip::{SipPlugin, SipRouter};
//! use rsipstack::sip::StatusCode;
//! use tx_di_core::BuildContext;
//!
//! // 启动前注册处理器
//! SipRouter::add_handler(Some("REGISTER"), 0, |mut tx| async move {
//!     println!("收到 REGISTER: {}", tx.original);
//!     tx.reply(StatusCode::OK).await?;
//!     Ok(())
//! });
//!
//! SipRouter::add_handler(Some("OPTIONS"), 0, |mut tx| async move {
//!     tx.reply(StatusCode::OK).await?;
//!     Ok(())
//! });
//!
//! // 启动 DI 框架
//! let mut ctx = BuildContext::new(Some("configs/di-config.toml"));
//! ctx.build_and_run().await.unwrap();
//! ```

mod config;
mod comp;
mod handler;
mod sender;

pub use config::*;
pub use comp::*;
pub use handler::SipRouter;
pub use sender::SipSender;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
