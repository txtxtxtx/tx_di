//! 诊断描述库（DID / DTC）
//!
//! 统一数据来源，被 ECU 仿真节点（应答内容）与前端（描述展示）共用。
//! 内置通用汽车诊断集，支持外部 JSON/TOML 加载追加。

pub mod builtin;
pub mod load;

use std::collections::HashMap;

pub use builtin::{DidMeta, DtcMeta};
pub use load::ExtDesc;

/// 描述库聚合
#[derive(Debug, Clone)]
pub struct DescDb {
    dids: HashMap<u16, DidMeta>,
    dtcs: HashMap<u32, DtcMeta>,
}

impl Default for DescDb {
    fn default() -> Self {
        Self::builtin()
    }
}

impl DescDb {
    /// 构造仅含内置集的描述库
    pub fn builtin() -> Self {
        DescDb {
            dids: builtin::builtin_dids(),
            dtcs: builtin::builtin_dtcs(),
        }
    }

    /// 从 JSON 文件加载并合并
    pub fn with_json(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let mut db = Self::builtin();
        let ext = ExtDesc::from_json(path)?;
        ext.merge_into(&mut db.dids, &mut db.dtcs);
        Ok(db)
    }

    /// 从 TOML 文件加载并合并
    pub fn with_toml(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let mut db = Self::builtin();
        let ext = ExtDesc::from_toml(path)?;
        ext.merge_into(&mut db.dids, &mut db.dtcs);
        Ok(db)
    }

    /// 查询 DID 元数据
    pub fn did_meta(&self, id: u16) -> Option<&DidMeta> {
        self.dids.get(&id)
    }

    /// 查询 DID 名称
    pub fn did_text(&self, id: u16) -> Option<&str> {
        self.dids.get(&id).map(|m| m.name.as_str())
    }

    /// 查询 DTC 文本
    pub fn dtc_text(&self, code: u32) -> Option<&str> {
        self.dtcs.get(&code).map(|m| m.text.as_str())
    }

    /// 列出全部内置/已加载 DTC 码（用于仿真"受支持 DTC"应答）
    pub fn supported_dtc_codes(&self) -> Vec<u32> {
        self.dtcs.keys().copied().collect()
    }

    /// 列出全部内置/已加载 DID
    pub fn supported_dids(&self) -> Vec<u16> {
        self.dids.keys().copied().collect()
    }

    /// 解析 DTC 状态掩码为置位的位名称列表
    pub fn dtc_status_bits(mask: u8) -> Vec<&'static str> {
        builtin::DTC_STATUS_BITS
            .iter()
            .filter(|(_, bit)| mask & bit != 0)
            .map(|(name, _)| *name)
            .collect()
    }

    /// 取得 DID 的默认（仿真初始）数据
    pub fn did_default_data(&self, id: u16) -> Option<Vec<u8>> {
        self.dids.get(&id).and_then(|m| m.default_data.clone())
    }
}
