//! 工程管理（.canproj）
//!
//! 把一套完整的上位机配置（适配器/ECU/ISO-TP/UDS 默认值、常用 DID/DTC、刷写参数）
//! 序列化为 JSON 工程文件，便于不同车型/ECU 之间切换与归档。

use crate::config::CanConfig;
use serde::{Deserialize, Serialize};

/// 工程中的刷写默认参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashProjectConfig {
    pub target_id: u32,
    pub security_level: u8,
    pub memory_address: u32,
    pub erase_before_download: bool,
}

impl Default for FlashProjectConfig {
    fn default() -> Self {
        FlashProjectConfig {
            target_id: 0x7E0,
            security_level: 0x01,
            memory_address: 0x0800_0000,
            erase_before_download: false,
        }
    }
}

/// 工程文件结构（.canproj = JSON）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// 工程名（车型/ECU 标识）
    pub name: String,
    /// 总线/适配器配置
    pub can: CanConfig,
    /// 默认诊断请求 ID（响应 = tx_id + 8）
    pub uds_tx_id: u32,
    /// 常用 DID 列表（一键诊断）
    pub recent_dids: Vec<u16>,
    /// 常用 DTC 列表
    pub recent_dtcs: Vec<u32>,
    /// 刷写默认参数
    pub flash: FlashProjectConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        ProjectConfig {
            name: "未命名工程".to_string(),
            can: CanConfig::default(),
            uds_tx_id: 0x7E0,
            recent_dids: vec![0xF190, 0xF195, 0xF18C],
            recent_dtcs: vec![],
            flash: FlashProjectConfig::default(),
        }
    }
}

impl ProjectConfig {
    /// 保存为 .canproj（JSON）文件
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, s)
    }

    /// 从 .canproj（JSON）文件加载
    pub fn load(path: &str) -> std::io::Result<ProjectConfig> {
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_roundtrip() {
        let mut p = ProjectConfig::default();
        p.name = "车型A".to_string();
        p.recent_dids = vec![0xF190, 0xF195];
        let dir = std::env::temp_dir();
        let path = dir.join(format!("txdi_proj_{}.canproj", std::process::id()));
        p.save(path.to_str().unwrap()).unwrap();
        let loaded = ProjectConfig::load(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.name, "车型A");
        assert_eq!(loaded.recent_dids, vec![0xF190, 0xF195]);
        assert_eq!(loaded.flash.target_id, 0x7E0);
        let _ = std::fs::remove_file(&path);
    }
}
