//! GB28181 设备注册表
//!
//! 线程安全的并发哈希表，存储已注册设备的状态。
//! 使用 `DashMap` 实现无锁并发读，适合高频心跳更新场景。
//!
//! 设备与通道的**数据类型**（`DeviceInfo` / `ChannelInfo` / `ChannelStatus`）
//! 已提取到 `tx_gb28181::device` 公共模块，此处通过 re-export 保持向后兼容。

use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};

// ── 从公共模块 re-export（向后兼容）─────────────────────────────────────────
pub use tx_gb28181::device::{ChannelInfo, ChannelStatus, DeviceInfo};

/// GB28181 设备注册表
///
/// 使用 `Arc<DashMap>` 可跨线程共享，无锁并发访问。
#[derive(Clone)]
pub struct DeviceRegistry {
    inner: Arc<DashMap<String, DeviceInfo>>,
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    // ── 注册/注销 ────────────────────────────────────────────────────────────

    /// 注册或更新设备
    pub fn register(&self, info: DeviceInfo) {
        let device_id = info.device_id.clone();
        let is_new = !self.inner.contains_key(&device_id);
        self.inner.insert(device_id.clone(), info);
        if is_new {
            info!(device_id = %device_id, "✅ 设备注册成功");
        } else {
            info!(device_id = %device_id, "🔄 设备注册刷新");
        }
    }

    /// 注销设备（REGISTER Expires: 0）
    pub fn unregister(&self, device_id: &str) -> bool {
        let removed = self.inner.remove(device_id).is_some();
        if removed {
            info!(device_id = %device_id, "🔌 设备主动注销");
        }
        removed
    }

    /// 设备下线（心跳超时）
    pub fn set_offline(&self, device_id: &str) {
        if let Some(mut dev) = self.inner.get_mut(device_id)
            && dev.online
        {
            warn!(device_id = %device_id, "⚠️ 设备心跳超时，标记离线");
            dev.online = false;
        }
    }

    // ── 心跳 ─────────────────────────────────────────────────────────────────

    /// 刷新设备心跳时间戳（收到 MESSAGE Keepalive 时调用）
    ///
    /// 返回：(刷新成功, 之前是否离线)
    pub fn refresh_heartbeat(&self, device_id: &str) -> bool {
        if let Some(mut dev) = self.inner.get_mut(device_id) {
            dev.refresh_heartbeat();
            if !dev.online {
                info!(device_id = %device_id, "🟢 设备重新上线");
                dev.online = true;
            }
            return true;
        }
        false
    }

    // ── 查询 ─────────────────────────────────────────────────────────────────

    /// 获取设备信息（克隆）
    pub fn get(&self, device_id: &str) -> Option<DeviceInfo> {
        self.inner.get(device_id).map(|r| r.clone())
    }

    /// 获取所有在线设备列表
    pub fn online_devices(&self) -> Vec<DeviceInfo> {
        self.inner
            .iter()
            .filter(|r| r.online)
            .map(|r| r.clone())
            .collect()
    }

    /// 获取所有设备数量
    pub fn total_count(&self) -> usize {
        self.inner.len()
    }

    /// 获取在线设备数量
    pub fn online_count(&self) -> usize {
        self.inner.iter().filter(|r| r.online).count()
    }

    /// 列出所有设备 ID
    pub fn device_ids(&self) -> Vec<String> {
        self.inner.iter().map(|r| r.device_id.clone()).collect()
    }

    /// 设备是否已注册
    pub fn is_registered(&self, device_id: &str) -> bool {
        self.inner.contains_key(device_id)
    }

    // ── 更新 ─────────────────────────────────────────────────────────────────

    /// 更新设备通道列表（收到 Catalog 响应时调用）
    pub fn update_channels(&self, device_id: &str, channels: Vec<ChannelInfo>) {
        if let Some(mut dev) = self.inner.get_mut(device_id) {
            info!(
                device_id = %device_id,
                channel_count = channels.len(),
                "📂 更新设备通道列表"
            );
            dev.channels = channels;
        }
    }

    /// 更新设备信息（收到 DeviceInfo 响应时调用）
    pub fn update_device_info(
        &self,
        device_id: &str,
        manufacturer: &str,
        model: &str,
        firmware: &str,
    ) {
        if let Some(mut dev) = self.inner.get_mut(device_id) {
            dev.manufacturer = manufacturer.to_string();
            dev.model = model.to_string();
            dev.firmware = firmware.to_string();
        }
    }

    // ── 超时检测 ─────────────────────────────────────────────────────────────

    /// 检查所有设备心跳超时，返回超时设备 ID 列表
    pub fn check_timeouts(&self, timeout_secs: u64) -> Vec<String> {
        self.inner
            .iter()
            .filter(|r| r.online && r.is_timeout(timeout_secs))
            .map(|r| r.device_id.clone())
            .collect()
    }
}
