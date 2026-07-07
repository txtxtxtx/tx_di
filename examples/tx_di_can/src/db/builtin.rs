//! DID / DTC 内置描述数据
//!
//! 内置通用汽车诊断描述集，供 ECU 仿真节点应答与前端描述展示共用。
//! 用户可通过 `DescDb::load_json` / `DescDb::load_toml` 追加自定义描述。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 数据标识符（DID）元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidMeta {
    /// DID（如 0xF190）
    pub id: u16,
    /// 人类可读名称
    pub name: String,
    /// 单位（无则 None）
    pub unit: Option<String>,
    /// 物理值系数：physical = raw * factor + offset
    pub factor: f64,
    /// 物理值偏移
    pub offset: f64,
    /// 原始字节长度（读/写时字节数）
    pub byte_len: usize,
    /// 值表（枚举文本映射，如 {0:"Off",1:"On"}）
    #[serde(default)]
    pub value_table: Option<HashMap<u32, String>>,
    /// ECU 仿真初始值（默认返回的数据），None 表示未提供
    #[serde(default)]
    pub default_data: Option<Vec<u8>>,
}

impl DidMeta {
    /// 将原始字节按大端解释为整数后换算为物理值
    pub fn physical(&self, raw: &[u8]) -> f64 {
        let mut v: u64 = 0;
        for &b in raw {
            v = (v << 8) | b as u64;
        }
        v as f64 * self.factor + self.offset
    }

    /// 若有值表则返回物理值的枚举文本
    pub fn value_text(&self, raw: &[u8]) -> Option<String> {
        let table = self.value_table.as_ref()?;
        let mut v: u64 = 0;
        for &b in raw {
            v = (v << 8) | b as u64;
        }
        table.get(&(v as u32)).cloned()
    }
}

/// 故障码（DTC）元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtcMeta {
    /// DTC 码（3 字节，如 0x000101 表示 P0101）
    pub code: u32,
    /// 人类可读文本
    pub text: String,
    /// 严重度（可选）
    #[serde(default)]
    pub severity: Option<String>,
}

/// 内置 DID 集
pub fn builtin_dids() -> HashMap<u16, DidMeta> {
    let mut m = HashMap::new();

    m.insert(
        0xF190,
        DidMeta {
            id: 0xF190,
            name: "Vehicle Identification Number".into(),
            unit: None,
            factor: 1.0,
            offset: 0.0,
            byte_len: 17,
            value_table: None,
            default_data: Some(b"1HGCM82633A004352".to_vec()),
        },
    );

    m.insert(
        0xF195,
        DidMeta {
            id: 0xF195,
            name: "ECU Software Version".into(),
            unit: None,
            factor: 1.0,
            offset: 0.0,
            byte_len: 4,
            value_table: None,
            default_data: Some(vec![0x01, 0x02, 0x03, 0x04]),
        },
    );

    m.insert(
        0xF18C,
        DidMeta {
            id: 0xF18C,
            name: "ECU Hardware Version".into(),
            unit: None,
            factor: 1.0,
            offset: 0.0,
            byte_len: 4,
            value_table: None,
            default_data: Some(vec![0x10, 0x20, 0x30, 0x40]),
        },
    );

    m.insert(
        0xF193,
        DidMeta {
            id: 0xF193,
            name: "Vehicle Manufacturer Spare Part Number".into(),
            unit: None,
            factor: 1.0,
            offset: 0.0,
            byte_len: 10,
            value_table: None,
            default_data: Some(b"SP-00012345".to_vec()),
        },
    );

    m.insert(
        0x0100,
        DidMeta {
            id: 0x0100,
            name: "Engine Speed".into(),
            unit: Some("rpm".into()),
            factor: 0.125,
            offset: 0.0,
            byte_len: 2,
            value_table: None,
            default_data: Some(vec![0x10, 0x68]), // 0x1068 = 4200 * 0.125
        },
    );

    m.insert(
        0x010B,
        DidMeta {
            id: 0x010B,
            name: "Vehicle Speed".into(),
            unit: Some("km/h".into()),
            factor: 1.0,
            offset: 0.0,
            byte_len: 1,
            value_table: None,
            default_data: Some(vec![0x38]), // 56 km/h
        },
    );

    m.insert(
        0x0121,
        DidMeta {
            id: 0x0121,
            name: "Ambient Temperature".into(),
            unit: Some("°C".into()),
            factor: 1.0,
            offset: -40.0,
            byte_len: 1,
            value_table: None,
            default_data: Some(vec![0x19]), // 25 °C
        },
    );

    m.insert(
        0x0200,
        DidMeta {
            id: 0x0200,
            name: "Ignition Status".into(),
            unit: None,
            factor: 1.0,
            offset: 0.0,
            byte_len: 1,
            value_table: Some(
                vec![(0u32, "Off".into()), (1, "Run".into()), (2, "Crank".into())]
                    .into_iter()
                    .collect(),
            ),
            default_data: Some(vec![0x01]),
        },
    );

    m
}

/// 内置 DTC 集（部分通用动力总成/底盘/车身故障码）
pub fn builtin_dtcs() -> HashMap<u32, DtcMeta> {
    let mut m = HashMap::new();

    let add = |m: &mut HashMap<u32, DtcMeta>, code: u32, text: &str, severity: Option<&str>| {
        m.insert(
            code,
            DtcMeta {
                code,
                text: text.into(),
                severity: severity.map(|s| s.into()),
            },
        );
    };

    add(&mut m, 0x000101, "P0101 Mass Air Flow Circuit Range/Performance", Some("medium"));
    add(&mut m, 0x000102, "P0102 Mass Air Flow Circuit Low", Some("medium"));
    add(&mut m, 0x000300, "P0300 Random/Multiple Cylinder Misfire Detected", Some("high"));
    add(&mut m, 0x000420, "P0420 Catalyst System Efficiency Below Threshold (Bank 1)", Some("high"));
    add(&mut m, 0x000701, "P0701 Transmission Control System Range/Performance", Some("medium"));
    add(&mut m, 0x00C021, "C0211 Wheel Speed Sensor Front Right Circuit", Some("high"));
    add(&mut m, 0x00C024, "C0241 ABS Hydraulic Pump Motor Circuit", Some("high"));
    add(&mut m, 0x008001, "B0001 Driver Frontal Stage 1 Deployment", Some("high"));
    add(&mut m, 0x000420, "U0420 Lost Communication With Battery Energy Control Module", Some("medium"));
    add(&mut m, 0x009001, "U0100 Lost Communication With ECM/PCM", Some("high"));

    m
}

/// DTC 状态掩码各位含义（ISO 14229-1 表）
pub const DTC_STATUS_BITS: &[(&str, u8)] = &[
    ("TestFailed", 0x01),
    ("TestFailedThisOperationCycle", 0x02),
    ("Pending", 0x04),
    ("Confirmed", 0x08),
    ("TestNotCompletedSinceLastClear", 0x10),
    ("TestFailedSinceLastClear", 0x20),
    ("TestNotCompletedThisOperationCycle", 0x40),
    ("WarningIndicatorRequested", 0x80),
];
