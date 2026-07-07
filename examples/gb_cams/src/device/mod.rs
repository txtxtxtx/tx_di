//! 设备业务层（单实例多通道）
//!
//! 本模块不再直接接触 SIP：注册 / 心跳 / INVITE / BYE 由
//! `tx_di_gb_dev::Gb28181Device` 统一处理。这里仅维护虚拟设备目录
//! （通道集合）、事件广播与媒体流生命周期，供
//! [`handler_impl::GbCamsHandler`]（实现 `DeviceHandler`）与 Web API 共享。

pub mod virtual_device;
pub mod handler_impl;

pub use virtual_device::{VirtualChannel, VirtualDevice};

use crate::config::GbCamsConfig;
use crate::device::handler_impl::MediaManager;
use crate::device::virtual_device::ChannelStatus;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::info;

/// 设备事件（供 API 层 SSE 推送）
#[derive(Debug, Clone, serde::Serialize)]
pub enum DeviceEvent {
    Registered { device_id: String },
    RegisterFailed { device_id: String, reason: String },
    Unregistered { device_id: String },
    Keepalive { device_id: String },
    Offline { device_id: String },
    CatalogQuery { device_id: String, sn: u32 },
    InviteReceived { device_id: String, channel_id: String, call_id: String },
    InviteEnded { device_id: String, call_id: String },
    /// PTZ 控制已下发到业务层
    Ptz { channel_id: String },
}

/// 全局设备业务管理器单例（不管理 SIP，仅业务目录 + 媒体 + 事件）
static INSTANCE: OnceLock<Arc<DeviceManager>> = OnceLock::new();

/// 设备业务管理器
///
/// 持有虚拟设备目录、事件广播与媒体流管理。SIP 注册 / 心跳 / 点播响应
/// 由 `Gb28181Device` 组件驱动，本管理器只负责业务状态与媒体生命周期。
pub struct DeviceManager {
    /// 虚拟设备目录（每个设备 = 一组通道）
    pub devices: Arc<DashMap<String, VirtualDevice>>,
    /// 业务配置（视频参数等）
    pub config: Arc<GbCamsConfig>,
    /// 事件广播
    event_tx: broadcast::Sender<DeviceEvent>,
    /// 媒体流管理（按通道 ID）
    pub media: MediaManager,
    /// 全局停止令牌（app 退出时取消所有媒体流）
    cancel: CancellationToken,
}

impl DeviceManager {
    /// 初始化全局单例（需在 `BuildContext::build()` 之前调用，仅依赖业务配置）
    pub fn init(config: Arc<GbCamsConfig>) {
        let (event_tx, _) = broadcast::channel(1024);
        let mgr = Arc::new(Self {
            devices: Arc::new(DashMap::new()),
            config,
            event_tx,
            media: MediaManager::new(),
            cancel: CancellationToken::new(),
        });
        let _ = INSTANCE.set(mgr);
    }

    /// 获取全局实例
    pub fn instance() -> Arc<DeviceManager> {
        INSTANCE
            .get()
            .expect("DeviceManager 尚未初始化，请确保 init() 已调用")
            .clone()
    }

    /// 订阅设备事件
    pub fn subscribe(&self) -> broadcast::Receiver<DeviceEvent> {
        self.event_tx.subscribe()
    }

    /// 发送事件
    pub fn emit(&self, event: DeviceEvent) {
        let _ = self.event_tx.send(event);
    }

    /// 添加设备（加入目录；注册由框架统一处理，无需独立 SIP 端口）
    pub fn add_device(
        &self,
        device_id: String,
        channels: Vec<(String, String)>,
        name: String,
    ) -> String {
        let vchannels: Vec<VirtualChannel> = channels
            .into_iter()
            .map(|(id, name)| VirtualChannel {
                channel_id: id,
                name,
                status: ChannelStatus::On,
            })
            .collect();
        let dev = VirtualDevice::new(device_id.clone(), vchannels, name, 0);
        self.devices.insert(device_id.clone(), dev);
        device_id
    }

    /// 删除设备（停止其所有通道媒体流）
    pub fn remove_device(&self, device_id: &str) -> bool {
        if let Some((_, dev)) = self.devices.remove(device_id) {
            for ch in &dev.channels {
                self.media.stop(&ch.channel_id);
            }
            self.emit(DeviceEvent::Unregistered {
                device_id: device_id.to_string(),
            });
            true
        } else {
            false
        }
    }

    pub fn get_device(&self, device_id: &str) -> Option<VirtualDevice> {
        self.devices.get(device_id).map(|r| r.clone())
    }

    pub fn all_devices(&self) -> Vec<VirtualDevice> {
        self.devices.iter().map(|r| r.clone()).collect()
    }

    /// 在线设备数（单实例已注册，目录非空即整体在线）
    pub fn online_count(&self) -> usize {
        self.devices.len()
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let total = self.devices.len();
        let channels: usize = self.devices.iter().map(|r| r.channels.len()).sum();
        (total, total, channels)
    }

    /// 触发设备目录"就绪"（单实例注册由 `Gb28181Device` 框架统一处理，
    /// 此处仅作兼容 API 的日志提示）
    pub fn start_all(&self) {
        info!(
            count = self.devices.len(),
            "设备目录已就绪，注册由 Gb28181Device 框架统一处理"
        );
    }

    /// 优雅停止：取消所有媒体流
    pub fn shutdown(&self) {
        self.cancel.cancel();
        self.media.stop_all();
    }
}
