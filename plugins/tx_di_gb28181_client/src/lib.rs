//! # tx_di_gb28181_client — GB28181 设备客户端插件
//!
//! 模拟 GB28181 前端设备（IPC、NVR）行为：注册、心跳、响应点播。
//!
//! ## 功能概览
//!
//! | 功能                   | 说明                                             |
//! |------------------------|--------------------------------------------------|
//! | **自动注册**           | 启动后自动向上级平台注册，处理 401 摘要认证      |
//! | **自动心跳**           | 定时发送 MESSAGE Keepalive，自动刷新注册         |
//! | **断线重连**           | 注册失败后指数退避重试，网络恢复后自动续注       |
//! | **目录响应**           | 响应平台 Catalog 查询，上报通道列表              |
//! | **点播响应**           | 接收 INVITE，发送 200 OK + SDP，管理媒体会话     |
//! | **设备信息响应**       | 响应 DeviceInfo 查询                             |
//! | **事件总线**           | 提供 `DeviceEvent` 供上层业务订阅                |
//!
//! ## 快速开始
//!
//! ```toml
//! [dependencies]
//! tx_di_gb28181_client = { path = "plugins/tx_di_gb28181_client" }
//! ```
//!
//! ```rust,no_run
//! use tx_di_gb28181_client::{Gb28181Device, DeviceEvent, ChannelConfig};
//! use tx_di_core::BuildContext;
//!
//! // 注册事件监听
//! Gb28181Device::on_event(|ev| async move {
//!     if let DeviceEvent::InviteAccepted { call_id, sdp_answer, .. } = ev {
//!         println!("点播会话建立 call_id={call_id}，开始推流...");
//!         // TODO: 启动媒体推流
//!     }
//!     Ok(())
//! });
//!
//! let mut ctx = BuildContext::new(Some("configs/gb28181-device.toml"));
//! ctx.build().await.unwrap();
//! // 框架会自动完成注册 + 心跳，无需手动操作
//! ```

mod channel;
mod client_plugin;
mod config;
mod device_handlers;
mod device_event;

pub use channel::ChannelConfig;
pub use client_plugin::Gb28181Device;
pub use config::Gb28181DeviceConfig;
pub use device_event::DeviceEvent;
