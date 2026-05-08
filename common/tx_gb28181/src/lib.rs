//! # tx_gb28181 — GB28181 公共库
//!
//! 供 `tx_di_gb28181`（服务端插件）、`tx_di_gb28181_client`（设备客户端插件）
//! 以及 `gb28181_admin`、`gb_cams` 等示例程序共同使用的 GB28181 工具集。
//!
//! ## 模块
//! - [`xml`]：MANSCDP XML 构建与解析（PTZ / 目录 / 录像 / 报警 / 时间同步…）
//! - [`sdp`]：SDP 构建与解析（点播 / 回放 / 对讲 / 抓拍…）

pub mod sdp;
pub mod xml;
