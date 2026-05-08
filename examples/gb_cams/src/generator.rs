//! GB28181 设备/通道随机生成器
//!
//! GB28181 设备ID编码规则（20位）：
//! - 省编码(2) + 市编码(2) + 县区编码(3) + 厂商编码(3) + 类型(2) + 序号(6)
//! - 例: 34020000001320000001
//!
//! 通道ID（20位）：
//! - 设备ID前18位 + 通道序号(2)
//! - 例: 34020000001320000001 → 通道1: 34020000001320000101

use rand::Rng;

/// 设备类型编码 131~199 表示类型为前端外围设备
pub const DEVICE_TYPE_IPC: &str = "132";  // IPC 摄像机

/// 111~130 表示类型为前端主设备
pub const DEVICE_TYPE_NVR: &str = "118";  // NVR 录像机

/// 生成设备 ID（20 位）
///
/// # 参数
/// - `prefix`: 前缀，通常为省+市+县区+厂商编码（如 "34020000001320"）
/// - `seq`: 序号（1-999999）
///
/// # 示例
/// ```
/// let id = generate_device_id("34020000001320", 1);
/// assert_eq!(id, "34020000001320000001");
/// ```
pub fn generate_device_id(prefix: &str, seq: u64) -> String {
    format!("{}{:06}", prefix, seq)
}

/// 生成通道 ID（20 位）
///
/// 通道 ID = 设备 ID 前18位 + 两位通道序号
pub fn generate_channel_id(device_id: &str, ch_seq: u32) -> String {
    let base = if device_id.len() >= 18 {
        &device_id[..18]
    } else {
        device_id
    };
    let mut id = format!("{}{:02}", base, ch_seq);
    // 替换 11-13位 为 132
    id.replace_range(11..14, DEVICE_TYPE_IPC);
    id

}

/// 随机设备前缀生成器
///
/// 根据给定的区域编码和厂商编码，生成设备 ID 前缀
pub fn random_device_prefix(region: &str, manufacturer: &str, device_type: &str) -> String {
    format!("{}{}{}", region, manufacturer, device_type)
}

/// 批量生成虚拟设备配置
///
/// # 参数
/// - `count`: 设备数量
/// - `channels_per_device`: 每个设备的通道数
/// - `prefix`: 设备 ID 前缀（14位，如 "34020000001320"）
/// - `base_seq`: 起始序号
///
/// # 返回
/// `(device_id, channel_ids)` 的列表
pub fn generate_devices(
    count: usize,
    channels_per_device: usize,
    prefix: &str,
    base_seq: u64,
) -> Vec<(String, Vec<String>, String)> {
    let mut result = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();

    for i in 0..count {
        let seq = base_seq + i as u64;
        let device_id = generate_device_id(prefix, seq);
        let channels: Vec<String> = (1..=channels_per_device)
            .map(|ch| generate_channel_id(&device_id, ch as u32))
            .collect();

        // 随机设备名称
        let name = format!("Camera-{:04}", rng.gen_range(1..=9999));
        result.push((device_id, channels, name));
    }

    result
}

/// 生成随机区域编码
pub fn random_region_code() -> &'static str {
    let regions = ["340200", "110100", "440300", "310100", "320100"];
    let idx = rand::thread_rng().gen_range(0..regions.len());
    regions[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_device_id() {
        let id = generate_device_id("34020000001320", 1);
        assert_eq!(id, "34020000001320000001");
        assert_eq!(id.len(), 20);
    }

    #[test]
    fn test_generate_channel_id() {
        let ch = generate_channel_id("34020000001320000001", 1);
        assert_eq!(ch, "34020000001320000101");
        assert_eq!(ch.len(), 20);
    }

    #[test]
    fn test_generate_devices() {
        let devices = generate_devices(3, 2, "34020000001320", 1);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].0, "34020000001320000001");
        assert_eq!(devices[0].1.len(), 2);
        assert_eq!(devices[0].1[0], "34020000001320000101");
        assert_eq!(devices[1].0, "34020000001320000002");
    }
}
