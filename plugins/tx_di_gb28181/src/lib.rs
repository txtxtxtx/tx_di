//! # tx_di_gb28181 — GB28181-2022 完整服务端插件
//!
//! 基于 `tx_di_sip` 构建的 GB28181 上级平台完整实现。
//!
//! ## 功能概览（GB28181-2022）
//!
//! | 功能                  | 说明                                          |
//! |-----------------------|-----------------------------------------------|
//! | **设备注册管理**      | REGISTER/注销/心跳，支持 SIP 摘要认证          |
//! | **心跳检测**          | MESSAGE Keepalive，超时自动注销               |
//! | **目录查询**          | Catalog 查询，完整解析通道列表（含经纬度等）    |
//! | **设备信息查询**      | DeviceInfo 查询/响应                           |
//! | **设备状态查询**      | DeviceStatus 查询/响应                         |
//! | **录像查询**          | RecordInfo 查询/响应，解析录像文件列表          |
//! | **实时点播**          | INVITE s=Play，联动 ZLM 分配 RTP 端口          |
//! | **历史回放**          | INVITE s=Playback，含时间范围 SDP              |
//! | **回放控制**          | 暂停/继续/快放/拖动（MediaControl）            |
//! | **BYE 挂断**          | 主动挂断会话，释放 ZLM 端口资源                |
//! | **PTZ 云台控制**      | 8 方向 + 变倍 + 聚焦 + 光圈（DeviceControl）  |
//! | **录像控制**          | 开始/停止录像                                  |
//! | **布撤防控制**        | 看守位设置/取消                                |
//! | **报警事件**          | NOTIFY 报警接收，触发 AlarmReceived 事件       |
//! | **报警订阅**          | SUBSCRIBE 订阅设备报警                         |
//! | **媒体状态通知**      | 设备推流结束通知（MediaStatus）                |
//! | **移动位置上报**      | MobilePosition GPS 定位通知                    |
//! | **ZLM 集成**          | HTTP API 客户端（openRtpServer/流管理/播放URL） |
//! | **事件总线**          | `Gb28181Event` 27 种事件，供上层业务订阅       |
//!
//! ## 快速开始
//!
//! ```toml
//! [dependencies]
//! tx_di_gb28181 = { path = "plugins/tx_di_gb28181" }
//! ```
//!
//! ## 配置文件
//!
//! ```toml
//! [gb28181_server_config]
//! platform_id            = "34020000002000000001"
//! realm                  = "3402000000"
//! sip_ip                 = "192.168.1.100"
//! heartbeat_timeout_secs = 120
//! enable_auth            = true
//! auth_password          = "12345678"
//!
//! [gb28181_server_config.media]
//! local_ip       = "192.168.1.100"
//!
//! [gb28181_server_config.zlm]
//! base_url = "http://127.0.0.1:8080"
//! secret   = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
//! ```
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use tx_di_gb28181::{Gb28181Server, Gb28181Event};
//! use tx_di_core::BuildContext;
//!
//! // 1. 订阅事件（在 build 之前注册）
//! Gb28181Server::on_event(|event| async move {
//!     match event {
//!         Gb28181Event::DeviceRegistered { device_id, .. } =>
//!             println!("设备上线: {}", device_id),
//!         Gb28181Event::CatalogReceived { device_id, channels, .. } =>
//!             println!("收到 {} 个通道", channels.len()),
//!         Gb28181Event::AlarmReceived { device_id, alarm_description, .. } =>
//!             println!("报警: {}", alarm_description),
//!         _ => {}
//!     }
//!     Ok(())
//! });
//!
//! // 2. 启动
//! let mut ctx = BuildContext::new(Some("configs/gb28181-server.toml"));
//! ctx.build().await.unwrap();
//!
//! let server = Gb28181Server::instance();
//!
//! // 3. 查询目录
//! server.query_catalog("34020000001320000001").await.unwrap();
//!
//! // 4. 发起实时点播（自动联动 ZLM）
//! let (call_id, urls) = server.invite("34020000001320000001", "34020000001320000001").await.unwrap();
//! println!("HLS: {}", urls.hls);
//! println!("RTSP: {}", urls.rtsp);
//!
//! // 5. PTZ 控制
//! use tx_di_gb28181::xml::{PtzCommand, PtzSpeed};
//! server.ptz_control("device_id", "channel_id",
//!     PtzCommand::Right(PtzSpeed::default())).await.unwrap();
//!
//! // 6. 录像查询
//! server.query_record_info(
//!     "device_id", "channel_id",
//!     "2024-01-01T00:00:00", "2024-01-02T00:00:00", 0
//! ).await.unwrap();
//!
//! // 7. 挂断
//! server.hangup(&call_id).await.unwrap();
//! ```

mod config;
mod device_registry;
mod event;
mod handlers;
mod plugin;
pub mod sdp;
pub mod xml;
pub mod media;

pub use config::{Gb28181ServerConfig, MediaConfig};
pub use device_registry::{ChannelInfo, ChannelStatus, DeviceInfo, DeviceRegistry};
pub use event::{Gb28181Event, subscribe as subscribe_events};
pub use handlers::Gb28181CmdType;
pub use plugin::{Gb28181Server, Gb28181ServerHandle, SessionInfo};
pub use sdp::{parse_sdp_ssrc, AudioCodec, AudioSessionInfo, SnapshotInfo};
pub use xml::{
    build_catalog_query_xml, build_device_info_query_xml, build_keepalive_xml,
    parse_xml_field, PtzCommand, PtzSpeed, PlaybackControl, TimeSyncInfo, ConfigType,
    ConfigItem, PresetInfo, CruiseInfo, CruiseTrack, CruisePoint,
    PtzPreciseParam, ZoomRect, GuardMode, StorageStatus, PtzPreciseStatus,
    TargetTrackMode, parse_storage_status, parse_cruise_track, parse_ptz_precise_status,
    parse_guard_info, GuardInfo,
};

// media 统一接口再导出
pub use media::{
    MediaBackend, MediaBackendConfig, BackendType,
    OpenRtpRequest, RtpServerHandle, PlayUrls,
    MediaStreamInfo, StreamProxyHandle, TcpMode,
    build_backend,
};
