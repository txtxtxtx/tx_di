//! # tx_di_gb_dev
//!
//! GB28181 设备端（UAC）插件，供 `gb_cams`（模拟设备）与平台级联下级（CascadeLower）复用。
//!
//! 设计目标：**业务零改造**——设备只需实现 [`DeviceHandler`] trait，由 [`Gb28181Device`]
//! 组件统一完成向上级平台的 `REGISTER` + 心跳 + 查询/点播响应，并按 [`GbDevConfig::version`]
//! 对出网报文做字符集编码。
//!
//! ## 四层架构位置
//!
//! - `L0 tx_di_sip`：纯净 SIP 栈（注册/发送原语，零 GB 语义）
//! - `L1 tx_gb28181`：纯国标协议（XML/SDP/编解码/版本策略）
//! - `L2b tx_di_gb_dev`（本 crate）：设备客户端（UAC）
//!
//! ## 快速接入
//!
//! ```rust,ignore
//! use tx_di_gb_dev::{Gb28181Device, DeviceHandler};
//! use async_trait::async_trait;
//!
//! struct MyCam;
//! #[async_trait]
//! impl DeviceHandler for MyCam {
//!     async fn on_catalog(&self, _sn: u32) -> Vec<(String, String)> {
//!         vec![("34020000001320000001".into(), "前门摄像机".into())]
//!     }
//! }
//! ```

mod config;
mod handler;
mod invite;
mod plugin;
mod register;

pub use config::GbDevConfig;
pub use handler::{DeviceHandler, NoopDeviceHandler};
pub use plugin::Gb28181Device;
