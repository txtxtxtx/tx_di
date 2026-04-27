//! # tx_di_can — CAN/CANFD 上位机插件
//!
//! 提供完整的 CAN/CANFD 上位机能力：
//!
//! ## 功能模块
//! - **多适配器抽象**：SocketCAN / PCAN / 仿真总线，统一 `CanAdapter` trait
//! - **ISO-TP (ISO 15765-2)**：单帧/首帧/连续帧/流控完整实现
//! - **UDS (ISO 14229)**：诊断服务全集（0x10/0x11/0x14/0x19/0x22/0x27/0x2E/0x34/0x36/0x37/0x3E 等）
//! - **刷写引擎**：标准 UDS 刷写流程（安全访问→请求下载→分块传输→退出传输→校验→复位）
//! - **CANFD 支持**：最大 64 字节载荷，支持 BRS/ESI
//! - **事件总线**：异步多订阅者，帧/诊断响应/刷写进度
//! - **仿真模式**：无需硬件即可测试（Simbus 全回环）
//!
//! ## 快速开始
//!
//! ```toml
//! # configs/can.toml
//! [can_config]
//! adapter     = "simbus"    # socketcan | pcan | simbus
//! interface   = "vcan0"
//! bitrate     = 500_000
//! isotp_tx_id = 0x7E0
//! isotp_rx_id = 0x7E8
//! ```
//!
//! ```rust,ignore
//! use tx_di_can::{CanPlugin, CanEvent, FlashConfig};
//! use tx_di_core::BuildContext;
//!
//! // 1. 订阅事件
//! CanPlugin::on_event(|ev| async move {
//!     match ev {
//!         CanEvent::UdsResponse { service, payload } => {
//!             println!("UDS {:02X} 响应: {:02X?}", service, payload);
//!         }
//!         CanEvent::FlashProgress { block_seq, total_blocks, bytes_sent, total_bytes } => {
//!             println!("刷写 {}/{} 块 ({} / {} bytes)",
//!                 block_seq, total_blocks, bytes_sent, total_bytes);
//!         }
//!         _ => {}
//!     }
//!     Ok(())
//! });
//!
//! // 2. 启动（配置文件路径默认为 configs/can.toml）
//! let mut ctx = BuildContext::new(Some("configs/can.toml"));
//! ctx.build().await.unwrap();
//!
//! // 3. UDS 诊断
//! let sw_version = CanPlugin::read_data(0x7DF, 0xF189).await.unwrap();
//! println!("ECU 版本: {:02X?}", sw_version);
//!
//! // 4. 刷写固件（seed→key 算法由业务提供）
//! CanPlugin::flash("firmware.bin", FlashConfig {
//!     target_id: 0x7E0,
//!     security_level: 0x01,
//!     memory_address: 0x08000000,
//!     ..Default::default()
//! }, |seed| {
//!     // 示例：简单取反算法（实际按 ECU 文档实现）
//!     seed.iter().map(|b| !b).collect()
//! }).await.unwrap();
//! ```

mod adapter;
mod config;
mod event;
mod frame;
mod flash;
mod isotp;
mod plugin;
mod uds;

pub use adapter::{CanAdapter, AdapterKind};

#[cfg(test)]
mod tests;
pub use config::CanConfig;
pub use event::{CanEvent, on_event};
pub use flash::{FlashConfig, FlashEngine, FlashProgress, FlashResult};
pub use frame::{CanFdFrame, CanFrame, FrameId, FrameKind};
pub use isotp::{IsoTpChannel, IsoTpConfig};
pub use plugin::CanPlugin;
pub use uds::{
    DtcRecord, NrcCode, SessionType, UdsClient, UdsError, UdsService,
};
