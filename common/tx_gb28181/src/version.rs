//! GB28181 协议版本（2016 / 2022）建模与字符集策略
//!
//! 每个设备 / 对端独立持有 [`GbVersion`]，支持平台混合组网同时对接 2016 与 2022 设备。
//!
//! - [`GbVersion::encoding`]：出网 XML 字符集声明（2016 → GB2312，2022 → GB18030）
//! - [`GbVersion::serialize`]：将内部 GB18030 声明的 XML 按本版本重声明并**真编码**为字节
//! - [`GbVersion::decode`]：容错解码入网字节为 Rust 字符串
//! - [`GbVersion::supports`]：按版本裁剪 2022 专有指令

use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};

/// GB28181 协议版本
///
/// 粒度：**每设备**。平台与设备端各自为每个对端保存独立的版本属性，
/// 以支持 2016 与 2022 设备混合组网。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GbVersion {
    /// GB/T 28181-2016（字符集 GB2312）
    V2016,
    /// GB/T 28181-2022（字符集 GB18030，默认）
    #[default]
    V2022,
}

impl GbVersion {
    /// 出网 XML 字符集声明
    pub fn encoding(&self) -> &'static str {
        match self {
            GbVersion::V2016 => "GB2312",
            GbVersion::V2022 => "GB18030",
        }
    }

    /// 对应的 encoding_rs 编码器
    fn encoder(&self) -> &'static Encoding {
        match self {
            // GB2312 是 GBK 的子集，encoding_rs 提供 GBK 编码器，
            // 对 GB2312 字符区间编码字节与 GB2312 完全一致。
            GbVersion::V2016 => encoding_rs::GBK,
            GbVersion::V2022 => encoding_rs::GB18030,
        }
    }

    /// 将内部以 `encoding="GB18030"` 声明的 XML 按本版本重新声明字符集，
    /// 并**真编码**为对应字符集的字节序列。
    ///
    /// 仅替换声明字符串不足以避免乱码——必须同时按目标字符集编码字节，
    /// 否则 2016 设备按 GB2312 解码 GB18030 字节会产生中文乱码。
    pub fn serialize(&self, xml: &str) -> Vec<u8> {
        // 1. 重写声明（内部 builder 统一产出 GB18030）
        let declared = xml.replace("GB18030", self.encoding());
        // 2. 按目标字符集真编码
        let (bytes, _, had_errors) = self.encoder().encode(&declared);
        if had_errors {
            tracing::warn!(
                version = ?self,
                "XML 中存在目标字符集无法编码的字符，已用替换符替代"
            );
        }
        bytes.into_owned()
    }

    /// 容错解码入网字节为 Rust 字符串。
    ///
    /// 优先按 GB18030（2022 超集，涵盖 GB2312/GBK）解码，
    /// 可正确还原两版设备上报的字节。
    pub fn decode(bytes: &[u8]) -> String {
        let (decoded, _, _) = encoding_rs::GB18030.decode(bytes);
        decoded.into_owned()
    }

    /// 该版本是否支持指定 MANSCDP 指令
    ///
    /// 2022 专有指令（`CruiseTrack` / `PtzPreciseStatus` / `GuardInfo`）
    /// 对 2016 设备不下发。
    pub fn supports(&self, cmd: &crate::cmd_type::Gb28181CmdType) -> bool {
        match self {
            GbVersion::V2022 => true,
            GbVersion::V2016 => !matches!(
                cmd,
                crate::cmd_type::Gb28181CmdType::CruiseTrack
                    | crate::cmd_type::Gb28181CmdType::PtzPreciseStatus
                    | crate::cmd_type::Gb28181CmdType::GuardInfo
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd_type::Gb28181CmdType;

    #[test]
    fn encoding_declaration() {
        assert_eq!(GbVersion::V2016.encoding(), "GB2312");
        assert_eq!(GbVersion::V2022.encoding(), "GB18030");
    }

    #[test]
    fn default_is_v2022() {
        assert_eq!(GbVersion::default(), GbVersion::V2022);
    }

    #[test]
    fn serialize_v2016_rewrites_declaration_and_encodes() {
        let xml = "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n<Name>测试</Name>";
        let expected = xml.replace("GB18030", "GB2312");
        let bytes = GbVersion::V2016.serialize(xml);
        // 字节按 GB2312 解码应能还原为「改写声明后的原文」
        let back = GbVersion::decode(&bytes);
        assert_eq!(back, expected, "GB2312 字节应可被 GB18030 解码还原");
        assert!(back.contains("encoding=\"GB2312\""), "声明应改为 GB2312");
        assert!(!back.contains("GB18030"), "不应残留 GB18030 声明");
    }

    #[test]
    fn serialize_v2022_keeps_gb18030() {
        let xml = "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n<Name>摄像机</Name>";
        let bytes = GbVersion::V2022.serialize(xml);
        assert_eq!(GbVersion::decode(&bytes), xml);
        assert!(GbVersion::decode(&bytes).contains("encoding=\"GB18030\""));
    }

    #[test]
    fn decode_roundtrip() {
        let xml = "<?xml version=\"1.0\" encoding=\"GB18030\"?>\r\n<Name>杭州监控</Name>";
        let bytes = GbVersion::V2022.serialize(xml);
        assert_eq!(GbVersion::decode(&bytes), xml);
    }

    #[test]
    fn v2022_supports_all_cmds() {
        for cmd in [
            Gb28181CmdType::Catalog,
            Gb28181CmdType::CruiseTrack,
            Gb28181CmdType::PtzPreciseStatus,
            Gb28181CmdType::GuardInfo,
        ] {
            assert!(GbVersion::V2022.supports(&cmd), "2022 应支持 {cmd}");
        }
    }

    #[test]
    fn v2016_excludes_2022_only_cmds() {
        assert!(GbVersion::V2016.supports(&Gb28181CmdType::Catalog));
        assert!(!GbVersion::V2016.supports(&Gb28181CmdType::CruiseTrack));
        assert!(!GbVersion::V2016.supports(&Gb28181CmdType::PtzPreciseStatus));
        assert!(!GbVersion::V2016.supports(&Gb28181CmdType::GuardInfo));
    }
}
