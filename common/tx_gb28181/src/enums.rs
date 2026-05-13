use crate::ChannelStatus;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

/// 设备编码类型
///
/// 在取值为行政区划时可为2、4、6、8位，其他情况取值为20位
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceIDType {
    Len2(String),
    Len4(String),
    Len6(String),
    Len8(String),
    Len20(String),
}
impl Serialize for DeviceIDType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DeviceIDType::Len2(s) => serializer.serialize_str(s),
            DeviceIDType::Len4(s) => serializer.serialize_str(s),
            DeviceIDType::Len6(s) => serializer.serialize_str(s),
            DeviceIDType::Len8(s) => serializer.serialize_str(s),
            DeviceIDType::Len20(s) => serializer.serialize_str(s),
        }
    }
}
impl<'de> Deserialize<'de> for DeviceIDType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.len() {
            2 => Ok(DeviceIDType::Len2(s)),
            4 => Ok(DeviceIDType::Len4(s)),
            6 => Ok(DeviceIDType::Len6(s)),
            8 => Ok(DeviceIDType::Len8(s)),
            20 => Ok(DeviceIDType::Len20(s)),
            _ => Err(serde::de::Error::invalid_length(
                s.len(),
                &"2, 4, 6, 8, or 20",
            )),
        }
    }
}
impl TryFrom<String> for DeviceIDType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() >= 2 {
            Ok(DeviceIDType::Len2(value))
        } else if value.len() >= 4 {
            Ok(DeviceIDType::Len4(value))
        } else if value.len() >= 6 {
            Ok(DeviceIDType::Len6(value))
        } else if value.len() >= 8 {
            Ok(DeviceIDType::Len8(value))
        } else if value.len() >= 20 {
            Ok(DeviceIDType::Len20(value))
        } else {
            Err("Invalid DeviceIDType")
        }
    }
}

/// 命令序列号类型
///
/// 最小值为1
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)] // 序列化时直接输出内部的 u32
pub struct SNType(u32);
impl<'de> Deserialize<'de> for SNType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value >= 1 {
            Ok(SNType(value))
        } else {
            Err(serde::de::Error::custom("SN must be >= 1"))
        }
    }
}
/// u32 转 SNType
impl TryFrom<u32> for SNType {
    type Error = &'static str;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >= 1 {
            Ok(SNType(value))
        } else {
            Err("SN must be >= 1")
        }
    }
}

/// 状态类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(untagged)]
pub enum StatusType {
    ON,
    OFF,
}
impl TryFrom<String> for StatusType {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "ON" => Ok(StatusType::ON),
            "OFF" => Ok(StatusType::OFF),
            _ => Err("Invalid StatusType"),
        }
    }
}

/// 结果类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ResultType {
    OK,
    ERROR,
}
impl TryFrom<String> for ResultType {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, <Self as TryFrom<String>>::Error> {
        match value.to_uppercase().as_str() {
            "OK" => Ok(ResultType::OK),
            "ERROR" => Ok(ResultType::ERROR),
            _ => Err("Invalid ResultType"),
        }
    }
}

/// 控制码类型 todo 待完善
///
/// 一个 16 进制字符串，长度为 16 个字符，对应 8 个字节（Bytes）的数据
///
/// - 1 固定头  A5
/// - 2 组合码  高4位 版本信息 通常是0 或 F；    低4位 校验位
/// - 3 地址低8位  云台地址的低位字节
/// - 4 指令码  PTZ命令的核心。用于区分具体动作，如向左、向右、放大等
/// - 5 & 6 地址高8位  数据1, 数据2 分别对应 水平速度 和 垂直速度
/// - 7 组合码2 高4位：通常为变焦速度等额外数据。  低4位：地址的高位
/// - 8 校验和 前7个字节的总和取模256（即 (byte1 + … + byte7) % 256）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PTZCmdType(String);
impl PTZCmdType {
    /// 从 8 字节数组构造
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        let hex_str = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        Self(hex_str)
    }

    /// 转换为 8 字节数组（若内容无效则返回 None）
    pub fn to_bytes(&self) -> Option<[u8; 8]> {
        if !Self::is_valid_hex(&self.0) {
            return None;
        }
        let mut bytes = [0u8; 8];
        for i in 0..8 {
            let byte_str = &self.0[2 * i..2 * i + 2];
            bytes[i] = u8::from_str_radix(byte_str, 16).ok()?;
        }
        Some(bytes)
    }

    /// 内部校验：长度16位，且全为十六进制字符（大小写均可）
    fn is_valid_hex(s: &str) -> bool {
        s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// 获取内部字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for PTZCmdType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_valid_hex(s) {
            Ok(PTZCmdType(s.to_uppercase()))
        } else {
            Err("PTZCmd must be 16 hexadecimal characters (0-9, A-F)")
        }
    }
}
impl TryFrom<String> for PTZCmdType {
    type Error = &'static str;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}
impl TryFrom<&str> for PTZCmdType {
    type Error = &'static str;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}
/// 自定义反序列化，确保格式正确
impl<'de> Deserialize<'de> for PTZCmdType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PTZCmdType::try_from(s).map_err(serde::de::Error::custom)
    }
}

/// 录像控制类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordType {
    Record,
    StopRecord,
}
impl TryFrom<String> for RecordType {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "RECORD" => Ok(RecordType::Record),
            "STOPRECORD" => Ok(RecordType::StopRecord),
            _ => Err("Invalid RecordType"),
        }
    }
}

/// 布防/撤防 控制类型 todo
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GuardType {
    SetGuard,
    ResetGuard,
}
impl TryFrom<String> for GuardType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "SetGuard" => Ok(GuardType::SetGuard),
            "ResetGuard" => Ok(GuardType::ResetGuard),
            _ => Err("Invalid GuardType"),
        }
    }
}

/// 摄像机结构类型，标识摄像机类型：1-球机；2-半球；3-固定枪机；4-遥控枪机；5-遥控半球；6-多目设备的全景/拼接通道；7-多目设备的分割通道。当为摄像机时可选。
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PTZType {
    /// 球机
    PTZ = 1,
    /// 半球
    DOME = 2,
    /// 固定枪机
    FixedBullet = 3,
    /// 遥控枪机
    RemoteBullet = 4,
    /// 遥控半球
    RemoteDome = 5,
    /// 多目设备的全景/拼接通道
    MultiObject = 6,
    /// 多目设备分割通道
    MultiObjectSplit = 7,
}

/// 摄像机光电成像类型。1-可见光成像；2-热成像；3-雷达成像；4-X光成像；5-深度光场成像；9-其他。可多值，用英文半角“/”分割。当为摄像机时可选
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PhotoelectricImagingType {
    /// 可见光
    VisibleLight = 1,
    /// 热成像
    Thermal = 2,
    /// 雷达
    Radar = 3,
    /// X光
    XRay = 4,
    /// 深度光场
    DepthLightField = 5,
    /// 其他
    Other = 9,
}
impl TryFrom<u8> for PhotoelectricImagingType {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(PhotoelectricImagingType::VisibleLight),
            2 => Ok(PhotoelectricImagingType::Thermal),
            3 => Ok(PhotoelectricImagingType::Radar),
            4 => Ok(PhotoelectricImagingType::XRay),
            5 => Ok(PhotoelectricImagingType::DepthLightField),
            9 => Ok(PhotoelectricImagingType::Other),
            _ => Err("Invalid PhotoelectricImagingType"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhotoelectricImagingTypes(pub Vec<PhotoelectricImagingType>);
impl Display for PhotoelectricImagingTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|t| (t.clone() as u8).to_string())
                .collect::<Vec<_>>()
                .join("/")
        )
    }
}
impl FromStr for PhotoelectricImagingTypes {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut vec = Vec::new();
        for part in s.split('/') {
            let val: u8 = part.parse().map_err(|_| "invalid number")?;
            vec.push(PhotoelectricImagingType::try_from(val)?);
        }
        Ok(Self(vec))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapturePositionType(String);

/// 摄像机安装位置室外、室内属性。1-室外、2-室内。当为摄像机时可选，缺省为 1。
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
pub enum RoomType {
    #[default]
    Outdoor = 1,
    Indoor = 2,
}
impl TryFrom<u8> for RoomType {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RoomType::Outdoor),
            2 => Ok(RoomType::Indoor),
            _ => Err("Invalid RoomType"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
pub enum SupplyLightType {
    /// 无补光
    #[default]
    NoneLight = 1,
    /// 红外补光
    InfraredLight = 2,
    /// 白光补光
    WhiteLight = 3,
    /// 激光补光
    LaserLight = 4,
    /// 其他补光
    OtherLight = 9,
}
impl TryFrom<u8> for SupplyLightType {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SupplyLightType::NoneLight),
            2 => Ok(SupplyLightType::InfraredLight),
            3 => Ok(SupplyLightType::WhiteLight),
            4 => Ok(SupplyLightType::LaserLight),
            9 => Ok(SupplyLightType::OtherLight),
            _ => Err("Invalid SupplyLightType"),
        }
    }
}

/// 摄像机监视方位(光轴方向)属性。1-东(西向东)、2-西(东向西)、3-南(北向南)、4-北(南向北)、5-东南(西北到东南)、6-东北(西南到东北)、7-西南(东北到西南)、8-西北(东南到西北)。当为摄像机时且为固定摄像机或设置看守位摄像机时可选。
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum DirectionType {
    /// 东(西向东)
    East = 1,
    /// 西(东向西)
    West = 2,
    /// 南(北向南)
    South = 3,
    /// 北(南向北)
    North = 4,
    /// 东南(西北到东南)
    SouthEast = 5,
    /// 东北(西南到东北)
    NorthEast = 6,
    /// 西南(东北到西南)
    SouthWest = 7,
    /// 西北(东南到西北)
    NorthWest = 8,
}
impl TryFrom<u8> for DirectionType {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DirectionType::East),
            2 => Ok(DirectionType::West),
            3 => Ok(DirectionType::South),
            4 => Ok(DirectionType::North),
            5 => Ok(DirectionType::SouthEast),
            6 => Ok(DirectionType::NorthEast),
            7 => Ok(DirectionType::SouthWest),
            8 => Ok(DirectionType::NorthWest),
            _ => Err("Invalid DirectionType"),
        }
    }
}

///  摄像机支持的分辨率，可多值，用英文半角“/”。分辨率取值应符合附录 G 中 SDP f 字段规定。当为摄像机时可选。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Resolution(String);

#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SVCSpaceSupportMode {
    /// 不支持
    NotSupport = 0,
    /// 1 级增强（1 个增强层）
    Level1Support = 1,
    /// 2 级增强（2 个增强层）
    Level2Support = 2,
    /// 3 级增强（3 个增强层）
    Level3Support = 3,
}
impl TryFrom<u8> for SVCSpaceSupportMode {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SVCSpaceSupportMode::NotSupport),
            1 => Ok(SVCSpaceSupportMode::Level1Support),
            2 => Ok(SVCSpaceSupportMode::Level2Support),
            3 => Ok(SVCSpaceSupportMode::Level3Support),
            _ => Err("Invalid SVCSpaceSupportMode"),
        }
    }
}

pub struct Info {
    /// 摄像机结构类型，标识摄像机类型：1-球机；2-半球；3-固定枪机；4-遥控枪机；5-遥控半球；6-多目设备的全景/拼接通道；7-多目设备的分割通道。当为摄像机时可选。
    pub ptz_type: Option<PTZType>,
    /// 摄像机光电成像类型。1-可见光成像；2-热成像；3-雷达成像；4-X光成像；5-深度光场成像；9-其他。可多值，用英文半角“/”分割。当为摄像机时可选
    pub photoelectric_imaging_type: Option<PhotoelectricImagingTypes>,
    /// 摄像机采集部位类型。应符合附录 O 中的规定。当为摄像机时可选。 todo 查看附录 O
    pub capture_position_type: Option<RoomType>,
    /// 摄像机补光属性。1-无补光；2-红外补光；3-白光补光；4-激光补光；9-其他。当为摄像机时可选，缺省为1。
    pub supply_light_type: Option<SupplyLightType>,
    /// 摄像机监视方位(光轴方向)属性。1-东(西向东)、2-西(东向西)、3-南(北向南)、4-北(南向北)、5-东南(西北到东南)、6-东北(西南到东北)、7-西南(东北到西南)、8-西北(东南到西北)。当为摄像机时且为固定摄像机或设置看守位摄像机时可选。
    pub direction_type: Option<DirectionType>,
    /// 摄像机支持的分辨率，可多值，用英文半角“/”。分辨率取值应符合附录 G 中 SDP f 字段规定。当为摄像机时可选。
    pub resolution: Option<Resolution>,
    /// 摄像机支持的码流编号列表，用于实时点播时指定码流编号（可选），多个取值间用英文半角“/”分割。如“0/1/2”，表示支持主码流，子码流 1，子码流 2，以此类推。
    pub stream_number_list: Option<String>,
    /// 下载倍速(可选),可多值,用英文半角“/”分割,如设备支持1,2,4倍速下载则应写为“1/2/4”
    pub download_speed: Option<String>,
    /// 空域编码能力，取值 0-不支持；1-1 级增强（1 个增强层）；2-2 级增强（2 个增强层）；3-3 级增强（3 个增强层）（可选）
    pub svc_space_support_mode: Option<SVCSpaceSupportMode>,
    /// todo 还有2个minerU图片没填写
    pub temp: Option<String>,
}

/// 目录项类型
pub struct ItemType {
    /// - 行政区划分时可为 2、4、6、8位
    /// - 其他情况为20位
    pub device_id: DeviceIDType,
    /// 设备名称
    pub name: String,
    /// 制造商名称
    pub manufacturer: String,
    /// 设备型号
    pub model: String,
    /// 行政区划代码 可选 2，4，6，8位
    pub civil_code: String,
    /// 警区
    pub block: Option<String>,
    /// 设备地址信息
    pub address: String,
    /// 是否有子设备（0=无，1=有）
    pub parental: u8,
    /// 父节点ID，可多值 / 分割
    pub parent_id: String,
    /// 注册方式（1=标准，2=基于口令的双向认证注册模式，3=基于数字证书的双向认证模式（高安全级别），4=基于数字证书的单向认证模式（高安全级别））
    ///
    /// 默认 1
    pub register_way: u8,
    /// 摄像机安全能力等级代码
    /// - A-GB 35114 前端设备安全能力A级
    /// - B-GB 35114 前端设备安全能力B级
    /// - C-GB 35114 前端设备安全能力C级
    pub security_level_code: Option<String>,
    /// 保密属性（0=非保密，1=保密）
    ///
    /// 默认 0
    pub secrecy: u8,
    /// 设备/系统 ipv4/ipv6 地址
    pub ip_address: Option<String>,
    /// 设备/系统 端口
    pub port: Option<u16>,
    /// 设备口令
    pub password: Option<String>,
    /// 设备状态（ON/OFF）
    pub status: StatusType,
    /// 经度坐标，一二类监控点位必选
    pub longitude: Option<f64>,
    /// 纬度坐标，一二类监控点位必选
    pub latitude: Option<f64>,
    /// 虚拟组织所属的业务分组 ID, 业务分组根据特定的业务需求制定, 一个业务分组包含一组特定的虚拟组织。
    pub business_group_id: Option<DeviceIDType>,
    pub info: Option<String>,
}

/// 文件目录类型
pub enum ItemFileType {}
/// PTZ精确控制类型
pub enum PTZPreciseType {}
/// OSD 配置类型
pub enum OSDCfgType {}
/// 视频参数属性类型
pub enum VideoParamAttributeCfgType {}
/// 移动设备位置类型
pub enum ItemMobilePositionType {}
/// 录像计划配置类型
pub enum VideoRecordPlanCfgType {}
/// 报警录像配置类型
pub enum VideoAlarmRecordCfgType {}
/// 视频画面遮挡配置类型
pub enum PictureMaskCfgType {}
/// 报警上报开关配置类型
pub enum AlarmReportCfgType {}
/// 基本参数配置类型
pub enum BasicParamCfgType {}
pub enum VideoParamOptCfgType {}
/// SVAC 编码配置类型
pub enum SVACEncodeCfgType {}
/// SVAC 解码配置类型
pub enum SVACDecodeCfgType {}
/// 画面翻转配置类型
pub enum FrameMirrorCfgType {}
/// 图像抓拍配置类型
pub enum SnapShotCfgType {}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试 PTZCmdType ==========================================================================
    #[test]
    fn test_valid_ptzcmd() {
        let cmd = PTZCmdType::try_from("A50F000800030CFF").unwrap();
        assert_eq!(cmd.as_str(), "A50F000800030CFF");
        let bytes = cmd.to_bytes().unwrap();
        assert_eq!(bytes, [0xA5, 0x0F, 0x00, 0x08, 0x00, 0x03, 0x0C, 0xFF]);
    }

    #[test]
    fn test_lowercase() {
        let cmd = PTZCmdType::try_from("a50f000800030c12").unwrap();
        assert_eq!(cmd.as_str(), "A50F000800030C12");
    }

    #[test]
    fn test_invalid_length() {
        assert!(PTZCmdType::try_from("A50F").is_err());
        assert!(PTZCmdType::try_from("A50F000800030C123").is_err());
    }

    #[test]
    fn test_invalid_chars() {
        assert!(PTZCmdType::try_from("A50F000800030C1G").is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = PTZCmdType::try_from("A50F000800030C12").unwrap();
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"A50F000800030C12\"");
        let deserialized: PTZCmdType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }
}
