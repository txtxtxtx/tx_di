//! # tx_gb28181 — GB28181 公共库
//!
//! 供 `tx_di_gb28181`（服务端插件）、`tx_di_gb28181_client`（设备客户端插件）
//! 以及 `gb28181_admin`、`gb_cams` 等示例程序共同使用的 GB28181 工具集。
//!
//! ## 模块
//! - [`device`]：设备与通道数据类型（`DeviceInfo` / `ChannelInfo` / `ChannelStatus`）
//! - [`cmd_type`]：协议指令枚举（`Gb28181CmdType`，覆盖 GB28181-2016/2022 全部 CmdType）
//! - [`event`]：事件类型（`Gb28181Event`）+ 全局广播基础设施（`subscribe` / `emit`）
//! - [`xml`]：MANSCDP XML 构建与解析（PTZ / 目录 / 录像 / 报警 / 时间同步…）
//! - [`sdp`]：SDP 构建与解析（点播 / 回放 / 对讲 / 抓拍…）

pub mod cmd_type;
pub mod device;
pub mod event;
pub mod sdp;
pub mod sip;
pub mod xml;

// ── 便捷再导出 ──────────────────────────────────────────────────────────────

pub use cmd_type::Gb28181CmdType;
pub use device::{ChannelInfo, ChannelStatus, DeviceInfo};
pub use event::{Gb28181Event, add_event_listener, emit, subscribe};
pub use sip::extract_user_from_sip_uri;
pub use xml::AlarmType;
