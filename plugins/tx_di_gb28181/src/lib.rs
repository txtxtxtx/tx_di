//! # tx_di_gb28181 — GB28181 服务端插件
//!
//! 基于 `tx_di_sip` 构建的 GB28181 上级平台（SIP 注册中心 + 目录管理 + 点播控制）实现。
//!
//! ## 功能概览
//!
//! | 功能                  | 说明                                    |
//! |-----------------------|-----------------------------------------|
//! | **设备注册管理**      | 接收/注销设备 REGISTER，维护注册表      |
//! | **心跳检测**          | 接收 MESSAGE Keepalive，超时自动注销     |
//! | **目录查询**          | 向设备发送 Catalog 查询，汇总通道列表    |
//! | **点播控制**          | 向设备下发 INVITE，管理媒体会话         |
//! | **事件总线**          | 提供 `Gb28181Event` 供上层业务订阅      |
//!
//! ## 快速开始
//!
//! ```toml
//! [dependencies]
//! tx_di_gb28181 = { path = "plugins/tx_di_gb28181" }
//! ```
//!
//! ```rust,no_run
//! use tx_di_gb28181::{Gb28181Server, Gb28181Event};
//! use tx_di_core::BuildContext;
//!
//! // 订阅事件（在 build 之前注册）
//! Gb28181Server::on_event(|event| async move {
//!     match event {
//!         Gb28181Event::DeviceRegistered { device_id, .. } =>
//!             println!("设备上线: {}", device_id),
//!         Gb28181Event::DeviceOffline { device_id } =>
//!             println!("设备离线: {}", device_id),
//!         _ => {}
//!     }
//!     Ok(())
//! });
//!
//! // 启动 DI 框架（自动初始化 SIP + GB28181）
//! let mut ctx = BuildContext::new(Some("configs/gb28181-server.toml"));
//! ctx.build().await.unwrap();
//!
//! // 向设备发起点播
//! let server = Gb28181Server::instance();
//! server.invite("34020000001320000001", "34020000001320000001", "192.168.1.10:10000").await.unwrap();
//! ```

mod config;
mod device_registry;
mod event;
mod handlers;
mod plugin;
pub mod sdp;
pub mod xml;

pub use config::{Gb28181ServerConfig, MediaConfig};
pub use device_registry::{DeviceInfo, DeviceRegistry};
pub use event::{Gb28181Event, Gb28181EventHandler};
pub use plugin::Gb28181Server;
pub use sdp::{build_invite_sdp, parse_sdp_ssrc};
pub use xml::{build_catalog_query_xml, build_device_info_query_xml, build_keepalive_xml, parse_xml_field};
