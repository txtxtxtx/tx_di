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

/// 摄像机光电成像类型。1-可见光成像；2-热成像；3-雷达成像；4-X光成像；5-深度光场成像；9-其他。可多值，用英文半角"/"分割。当为摄像机时可选
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

/// 摄像机光电成像类型。1-可见光成像；2-热成像；3-雷达成像；4-X光成像；5-深度光场成像；9-其他。可多值，用英文半角"/"分割。当为摄像机时可选
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
    Infrared = 2,
    /// 白光补光
    White = 3,
    /// 激光补光
    Laser = 4,
    /// 其他补光
    Other = 9,
}
impl TryFrom<u8> for SupplyLightType {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SupplyLightType::NoneLight),
            2 => Ok(SupplyLightType::Infrared),
            3 => Ok(SupplyLightType::White),
            4 => Ok(SupplyLightType::Laser),
            9 => Ok(SupplyLightType::Other),
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

///  摄像机支持的分辨率，可多值，用英文半角"/"。分辨率取值应符合附录 G 中 SDP f 字段规定。当为摄像机时可选。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Resolution(String);

#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SVCSpaceSupportMode {
    /// 不支持
    UnSupport = 0,
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
            0 => Ok(SVCSpaceSupportMode::UnSupport),
            1 => Ok(SVCSpaceSupportMode::Level1Support),
            2 => Ok(SVCSpaceSupportMode::Level2Support),
            3 => Ok(SVCSpaceSupportMode::Level3Support),
            _ => Err("Invalid SVCSpaceSupportMode"),
        }
    }
}

/// 时域编码能力，取值 0-不支持；1-1 级增强（1 个增强层）；2-2 级增强（2 个增强层）；3-3 级增强（3 个增强层）（可选）
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SVCTimeSupportMode {
    /// 不支持
    NotSupport = 0,
    /// 1 级增强（1 个增强层）
    Level1Support = 1,
    /// 2 级增强（2 个增强层）
    Level2Support = 2,
    /// 3 级增强（3 个增强层）
    Level3Support = 3,
}
impl TryFrom<u8> for SVCTimeSupportMode {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SVCTimeSupportMode::NotSupport),
            1 => Ok(SVCTimeSupportMode::Level1Support),
            2 => Ok(SVCTimeSupportMode::Level2Support),
            3 => Ok(SVCTimeSupportMode::Level3Support),
            _ => Err("Invalid SVCTimeSupportMode"),
        }
    }
}

/// SVC-SI 能力，取值 0-不支持；1-支持（可选）
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SVCSSIMode {
    /// 不支持
    NotSupport = 0,
    /// 支持
    Support = 1,
}
impl TryFrom<u8> for SVCSSIMode {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SVCSSIMode::NotSupport),
            1 => Ok(SVCSSIMode::Support),
            _ => Err("Invalid SVCSSIMode"),
        }
    }
}

/// 移动采集设备类型
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MobileDeviceType {
    MobileRobot = 1,
    BodyCamera = 2,
    SingleSoldier = 3,
    VehicleVideo = 4,
    Drone = 5,
    Other = 9,
}

/// 监控点位类型
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PointType {
    Class1 = 1,
    Class2 = 2,
    Class3 = 3,
    Other = 9,
}

// ---------------------------------------------------------------------------
// 光电成像类型（单值枚举）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PhotoelectricImagingTypeValue {
    Visible = 1,
    Thermal = 2,
    Radar = 3,
    XRay = 4,
    DepthLightField = 5,
    Other = 9,
}

// ---------------------------------------------------------------------------
// 卡口功能类型（单值枚举，注意序列化为两位数字符串）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionTypeValue {
    Face = 1,
    Person = 2,
    Vehicle = 3,
    NonMotorVehicle = 4,
    Object = 5,
    Other = 99,
}
impl Serialize for FunctionTypeValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = format!("{:02}", self.clone() as u8);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for FunctionTypeValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let num: u8 = s.parse().map_err(serde::de::Error::custom)?;
        match num {
            1 => Ok(FunctionTypeValue::Face),
            2 => Ok(FunctionTypeValue::Person),
            3 => Ok(FunctionTypeValue::Vehicle),
            4 => Ok(FunctionTypeValue::NonMotorVehicle),
            5 => Ok(FunctionTypeValue::Object),
            99 => Ok(FunctionTypeValue::Other),
            _ => Err(serde::de::Error::custom(format!("invalid FunctionTypeValue: {}", num))),
        }
    }
}

/// 卡口功能类型列表（如 "01/03/99"）
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionTypes(pub Vec<FunctionTypeValue>);

impl Serialize for FunctionTypes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.0.iter()
            .map(|v| format!("{:02}", v.clone() as u8))
            .collect::<Vec<_>>()
            .join("/");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for FunctionTypes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let parts = s.split('/');
        let mut values = Vec::new();
        for part in parts {
            let num: u8 = part.parse().map_err(serde::de::Error::custom)?;
            let value = match num {
                1 => FunctionTypeValue::Face,
                2 => FunctionTypeValue::Person,
                3 => FunctionTypeValue::Vehicle,
                4 => FunctionTypeValue::NonMotorVehicle,
                5 => FunctionTypeValue::Object,
                99 => FunctionTypeValue::Other,
                _ => return Err(serde::de::Error::custom(format!("invalid FunctionTypeValue: {}", num))),
            };
            values.push(value);
        }
        Ok(FunctionTypes(values))
    }
}

/// 码流编号列表（如 "0/1/2"）
#[derive(Debug, Clone, PartialEq)]
pub struct StreamNumberList(pub Vec<u8>);

impl Serialize for StreamNumberList {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.0.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("/");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for StreamNumberList {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let parts = s.split('/');
        let values = parts.map(|p| p.parse::<u8>().map_err(serde::de::Error::custom))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(StreamNumberList(values))
    }
}
/// 下载倍速列表（如 "1/2/4"）
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadSpeed(pub Vec<u8>);

impl Serialize for DownloadSpeed {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.0.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("/");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for DownloadSpeed {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let parts = s.split('/');
        let values = parts.map(|p| p.parse::<u8>().map_err(serde::de::Error::custom))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DownloadSpeed(values))
    }
}

/// SSVC 比例（如 "4:3"）
#[derive(Debug, Clone, PartialEq)]
pub struct Ratio {
    pub numerator: u32,
    pub denominator: u32,
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.numerator, self.denominator)
    }
}

impl FromStr for Ratio {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("invalid ratio format".to_string());
        }
        let numerator = parts[0].parse().map_err(|_| "invalid numerator".to_string())?;
        let denominator = parts[1].parse().map_err(|_| "invalid denominator".to_string())?;
        Ok(Ratio { numerator, denominator })
    }
}

/// SSVC 比例列表（如 "4:3/2:1/4:1"）
#[derive(Debug, Clone, PartialEq)]
pub struct SSVCRatioSupportList(pub Vec<Ratio>);

impl Serialize for SSVCRatioSupportList {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.0.iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join("/");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SSVCRatioSupportList {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let parts = s.split('/');
        let ratios = parts.map(|p| p.parse::<Ratio>().map_err(serde::de::Error::custom))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SSVCRatioSupportList(ratios))
    }
}

// ---------------------------------------------------------------------------
// 简单 newtype 包装（用于语义化字符串）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncodeType(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialClassification(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrassrootsCode(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MAC(pub String); // 格式 "XX-XX-XX-XX-XX-XX"

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagementUnit(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactInfo(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCommonName(pub String);

// 安装时间使用字符串（ISO8601），也可替换为 chrono::NaiveDateTime
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstallTime(pub String);

/// 目录项中摄像机的详细参数配置（GB/T 28181-2022 附录 A.2.1.9）
///
/// Info 结构体（对应 <Info> 元素）
/// 对应于 Catalog `<Item>` 中的 `<Info>` 子元素，包含摄像机的结构类型、
/// 光电成像、补光、方位、分辨率、码流、SVC 编码能力等扩展信息。
/// 所有字段均为可选，仅在摄像机节点中存在。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    #[serde(rename = "PTZType")]
    pub ptz_type: Option<PTZType>,

    #[serde(rename = "PhotoelectricImagingType")]
    pub photoelectric_imaging_type: Option<PhotoelectricImagingTypes>,

    #[serde(rename = "CapturePositionType")]
    pub capture_position_type: Option<CapturePositionType>,

    #[serde(rename = "RoomType")]
    pub room_type: Option<RoomType>,

    #[serde(rename = "SupplyLightType")]
    pub supply_light_type: Option<SupplyLightType>,

    #[serde(rename = "DirectionType")]
    pub direction_type: Option<DirectionType>,

    #[serde(rename = "Resolution")]
    pub resolution: Option<Resolution>,

    #[serde(rename = "StreamNumberList")]
    pub stream_number_list: Option<StreamNumberList>,

    #[serde(rename = "DownloadSpeed")]
    pub download_speed: Option<DownloadSpeed>,

    #[serde(rename = "SVCSpaceSupportMode")]
    pub svc_space_support_mode: Option<SVCSpaceSupportMode>,

    #[serde(rename = "SVCTimeSupportMode")]
    pub svc_time_support_mode: Option<SVCTimeSupportMode>,

    #[serde(rename = "SSVCRatioSupportList")]
    pub ssvc_ratio_support_list: Option<SSVCRatioSupportList>,

    #[serde(rename = "MobileDeviceType")]
    pub mobile_device_type: Option<MobileDeviceType>,

    #[serde(rename = "HorizontalFieldAngle")]
    pub horizontal_field_angle: Option<f64>,

    #[serde(rename = "VerticalFieldAngle")]
    pub vertical_field_angle: Option<f64>,

    #[serde(rename = "MaxViewDistance")]
    pub max_view_distance: Option<f64>,

    #[serde(rename = "GrassrootsCode")]
    pub grassroots_code: GrassrootsCode,

    #[serde(rename = "PointType")]
    pub point_type: Option<PointType>,

    #[serde(rename = "PointCommonName")]
    pub point_common_name: Option<PointCommonName>,

    #[serde(rename = "MAC")]
    pub mac: Option<MAC>,

    #[serde(rename = "FunctionType")]
    pub function_type: Option<FunctionTypes>,

    #[serde(rename = "EncodeType")]
    pub encode_type: Option<EncodeType>,

    #[serde(rename = "InstallTime")]
    pub install_time: Option<InstallTime>,

    #[serde(rename = "ManagementUnit")]
    pub management_unit: Option<ManagementUnit>,

    #[serde(rename = "ContactInfo")]
    pub contact_info: Option<ContactInfo>,

    #[serde(rename = "RecordSaveDays")]
    pub record_save_days: Option<i32>,

    #[serde(rename = "IndustrialClassification")]
    pub industrial_classification: Option<IndustrialClassification>,
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
    ///
    pub info: Option<Info>,
}

/// 文件目录类型
pub struct ItemFileType {

}
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
