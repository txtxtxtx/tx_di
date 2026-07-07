//! 描述库外部加载（JSON / TOML）
//!
//! 外部文件可包含 `dids` 与 `dtcs` 两个数组，加载后与内置集合并（相同 ID/码覆盖内置）。

use super::builtin::{DidMeta, DtcMeta};
use std::collections::HashMap;
use std::path::Path;

/// 外部描述文件结构（JSON / TOML 通用）
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct ExtDesc {
    #[serde(default)]
    pub dids: Vec<DidMeta>,
    #[serde(default)]
    pub dtcs: Vec<DtcMeta>,
}

impl ExtDesc {
    /// 从 JSON 文件加载
    pub fn from_json(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let v: ExtDesc = serde_json::from_str(&content)?;
        Ok(v)
    }

    /// 从 TOML 文件加载
    pub fn from_toml(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let v: ExtDesc = toml::from_str(&content)?;
        Ok(v)
    }

    /// 合并进现有映射
    pub fn merge_into(&self, dids: &mut HashMap<u16, DidMeta>, dtcs: &mut HashMap<u32, DtcMeta>) {
        for d in &self.dids {
            dids.insert(d.id, d.clone());
        }
        for d in &self.dtcs {
            dtcs.insert(d.code, d.clone());
        }
    }
}
