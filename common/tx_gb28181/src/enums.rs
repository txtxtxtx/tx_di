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
impl Default for DeviceIDType {
    fn default() -> Self {
        Self::Len20(String::new())
    }
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
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[serde(untagged)]
pub enum StatusType {
    #[default]
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

impl StatusType {
    /// 返回状态对应的字符串表示（"ON" / "OFF"）
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusType::ON => "ON",
            StatusType::OFF => "OFF",
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
            "SETGUARD" => Ok(GuardType::SetGuard),
            "RESETGUARD" => Ok(GuardType::ResetGuard),
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

/// 采集部位类型（CapturePositionType）
///
/// 摄像机采集部位类型，应符合 GB/T 28181-2022 附录 O 中的规定。
/// 当为摄像机时可选。
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
/// 摄像机补光属性（SupplyLightType）
///
/// 标识摄像机所采用的补光方式：
/// - `1` — 无补光（缺省值）
/// - `2` — 红外补光
/// - `3` — 白光补光
/// - `4` — 激光补光
/// - `9` — 其他补光
///
/// 当为摄像机时可选，缺省为 `1`。
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
/// 移动采集设备类型（MobileDeviceType）
///
/// 标识移动采集设备的类型：
/// - `1` — 移动机器人
/// - `2` — 佩戴式
/// - `3` — 单兵
/// - `4` — 车载视频
/// - `5` — 无人机
/// - `9` — 其他
///
/// 当为移动采集设备时可选。
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
/// 监控点位类型（PointType）
///
/// 标识监控点位的等级分类：
/// - `1` — 一类监控点位
/// - `2` — 二类监控点位
/// - `3` — 三类监控点位
/// - `9` — 其他
///
/// 当为摄像机时可选。
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

/// 光电成像类型单值（PhotoelectricImagingTypeValue）
///
/// 摄像机光电成像类型的单值枚举，取值：
/// - `1` — 可见光成像
/// - `2` — 热成像
/// - `3` — 雷达成像
/// - `4` — X光成像
/// - `5` — 深度光场成像
/// - `9` — 其他
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

/// 卡口功能类型单值（FunctionTypeValue）
///
/// 标识卡口设备所支持的单一功能类型，序列化为两位数字符串：
/// - `01` — 人脸
/// - `02` — 人体
/// - `03` — 机动车
/// - `04` — 非机动车
/// - `05` — 物体
/// - `99` — 其他
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
/// 卡口功能类型列表（FunctionTypes）
///
/// 标识卡口设备所支持的功能类型列表，序列化为 "/" 分割的两位数字符串。
/// 如 `"01/03/99"` 表示支持人脸、机动车和其他。
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
/// 码流编号列表（StreamNumberList）
///
/// 摄像机支持的码流编号列表，用于实时点播时指定码流编号。
/// 序列化为 "/" 分割的数字字符串，如 `"0/1/2"` 表示支持主码流、子码流 1、子码流 2。
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
/// 下载倍速列表（DownloadSpeed）
///
/// 设备支持的下载倍速范围，序列化为 "/" 分割的数字字符串。
/// 如 `"1/2/4"` 表示支持 1、2、4 倍速下载。
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

/// SSVC 分辨率比例（Ratio）
///
/// 表示 SSVC 编码中的宽高比，格式为 `"宽:高"`，如 `"4:3"`。
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

/// SSVC 比例支持列表（SSVCRatioSupportList）
///
/// 标识设备支持的 SSVC（空域可伸缩视频编码）分辨率比例列表。
/// 序列化为 "/" 分割的比例字符串，如 `"4:3/2:1/4:1"`。
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

/// 编码类型（EncodeType）
///
/// 摄像机支持的编码类型，多个编码类型用英文半角 "/" 分割。
/// 如 `"H.264/H.265/MJPEG"`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncodeType(pub String);

/// 行业分类代码（IndustrialClassification）
///
/// 摄像机所属的行业分类编码，应符合 GB/T 4754—2017 的规定。
/// 如 `"I1300"` 表示软件和信息技术服务业。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialClassification(pub String);

/// 基层组织代码（GrassrootsCode）
///
/// 摄像机所属基层组织的编码，使用行政区划代码标识。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GrassrootsCode(pub String);

/// MAC 地址（MAC）
///
/// 设备的 MAC 地址，格式为 `"XX-XX-XX-XX-XX-XX"`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MAC(pub String);

/// 管理单位（ManagementUnit）
///
/// 摄像机所属的管理单位名称或编码。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagementUnit(pub String);

/// 联系人信息（ContactInfo）
///
/// 设备/监控点位的联系人信息，如电话号码等。多值用 / 分割
/// 一类监控点必填
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactInfo(pub String);

/// 监控点位常用名称（PointCommonName）
///
/// 监控点位的常用/通俗名称，便于日常识别和管理。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCommonName(pub String);

/// 安装时间（InstallTime）
///
/// 摄像机的安装日期时间，使用 ISO 8601 格式字符串（如 `"2024-01-01T00:00:00"`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstallTime(pub String);

/// 目录项中摄像机的详细参数配置（GB/T 28181-2022 附录 A.2.1.9）
///
/// 对应于 Catalog `<Item>` 中的 `<Info>` 子元素，包含摄像机的结构类型、
/// 光电成像、补光、方位、分辨率、码流、SVC 编码能力、移动设备类型、
/// 视场角、监控点位、编码类型、安装信息等扩展参数。
/// 所有字段均为可选，仅在摄像机节点中存在。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Info {
    /// 摄像机结构类型（PTZType）
    ///
    /// 标识摄像机类型：
    /// - `1` — 球机
    /// - `2` — 半球
    /// - `3` — 固定枪机
    /// - `4` — 遥控枪机
    /// - `5` — 遥控半球
    /// - `6` — 多目设备的全景/拼接通道
    /// - `7` — 多目设备的分割通道
    ///
    /// 当为摄像机时可选。
    #[serde(rename = "PTZType")]
    pub ptz_type: Option<PTZType>,

    /// 摄像机光电成像类型（PhotoelectricImagingType）
    ///
    /// 标识摄像机所采用的光电成像方式：
    /// - `1` — 可见光成像
    /// - `2` — 热成像
    /// - `3` — 雷达成像
    /// - `4` — X光成像
    /// - `5` — 深度光场成像
    /// - `9` — 其他
    ///
    /// 可多值，用英文半角 "/" 分割。当为摄像机时可选。
    #[serde(rename = "PhotoelectricImagingType")]
    pub photoelectric_imaging_type: Option<PhotoelectricImagingTypes>,

    /// 摄像机采集部位类型（CapturePositionType）
    ///
    /// 标识摄像机采集部位类型，应符合 GB/T 28181-2022 附录 O 中的规定。
    /// 当为摄像机时可选。
    #[serde(rename = "CapturePositionType")]
    pub capture_position_type: Option<CapturePositionType>,

    /// 摄像机安装位置（RoomType）
    ///
    /// 标识摄像机安装位置是室内还是室外：
    /// - `1` — 室外（缺省值）
    /// - `2` — 室内
    ///
    /// 当为摄像机时可选，缺省为 `1`。
    #[serde(rename = "RoomType")]
    pub room_type: Option<RoomType>,

    /// 摄像机补光属性（SupplyLightType）
    ///
    /// 标识摄像机所采用的补光方式：
    /// - `1` — 无补光（缺省值）
    /// - `2` — 红外补光
    /// - `3` — 白光补光
    /// - `4` — 激光补光
    /// - `9` — 其他补光
    ///
    /// 当为摄像机时可选，缺省为 `1`。
    #[serde(rename = "SupplyLightType")]
    pub supply_light_type: Option<SupplyLightType>,

    /// 摄像机监视方位/光轴方向（DirectionType）
    ///
    /// 标识固定摄像机或设置看守位摄像机的光轴方向：
    /// - `1` — 东（西向东）
    /// - `2` — 西（东向西）
    /// - `3` — 南（北向南）
    /// - `4` — 北（南向北）
    /// - `5` — 东南（西北到东南）
    /// - `6` — 东北（西南到东北）
    /// - `7` — 西南（东北到西南）
    /// - `8` — 西北（东南到西北）
    ///
    /// 当为摄像机且为固定摄像机或设置看守位摄像机时可选。
    #[serde(rename = "DirectionType")]
    pub direction_type: Option<DirectionType>,

    /// 摄像机支持的分辨率列表（Resolution）
    ///
    /// 可多值，用英文半角 "/" 分割。
    /// 分辨率取值应符合附录 G 中 SDP `f` 字段规定。
    #[serde(rename = "Resolution")]
    pub resolution: Option<Resolution>,

    /// 摄像机支持的码流编号列表（StreamNumberList）
    ///
    /// 用于实时点播时指定码流编号，多个取值间用英文半角 "/" 分割。
    /// 如 `"0/1/2"` 表示支持主码流、子码流 1、子码流 2，以此类推。
    #[serde(rename = "StreamNumberList")]
    pub stream_number_list: Option<StreamNumberList>,

    /// 下载倍速列表（DownloadSpeed）
    ///
    /// 标识设备支持的下载倍速范围，可多值，用英文半角 "/" 分割。
    /// 如设备支持 1、2、4 倍速下载则应写为 `"1/2/4"`。
    #[serde(rename = "DownloadSpeed")]
    pub download_speed: Option<DownloadSpeed>,

    /// 空域编码能力（SVCSpaceSupportMode）
    ///
    /// 标识设备对空域 SVC 增强层的支持能力：
    /// - `0` — 不支持
    /// - `1` — 1 级增强（1 个增强层）
    /// - `2` — 2 级增强（2 个增强层）
    /// - `3` — 3 级增强（3 个增强层）
    ///
    #[serde(rename = "SVCSpaceSupportMode")]
    pub svc_space_support_mode: Option<SVCSpaceSupportMode>,

    /// 时域编码能力（SVCTimeSupportMode）
    ///
    /// 标识设备对时域 SVC 增强层的支持能力：
    /// - `0` — 不支持
    /// - `1` — 1 级增强（1 个增强层）
    /// - `2` — 2 级增强（2 个增强层）
    /// - `3` — 3 级增强（3 个增强层）
    ///
    #[serde(rename = "SVCTimeSupportMode")]
    pub svc_time_support_mode: Option<SVCTimeSupportMode>,

    /// SSVC 争强层与基本层的比例支持列表（SSVCRatioSupportList）
    ///
    /// 标识设备支持的 SSVC（空域可伸缩视频编码）分辨率比例。
    /// 多个比例用英文半角 "/" 分割，每个比例格式为 `"宽:高"`。
    /// 如 `"4:3/2:1/4:1"`。
    #[serde(rename = "SSVCRatioSupportList")]
    pub ssvc_ratio_support_list: Option<SSVCRatioSupportList>,

    /// 移动采集设备类型（MobileDeviceType）
    ///
    /// 标识移动采集设备的类型：
    /// - `1` — 移动机器人
    /// - `2` — 佩戴式
    /// - `3` — 单兵
    /// - `4` — 车载视频
    /// - `5` — 无人机
    /// - `9` — 其他
    ///
    /// 当为移动采集设备时必选。
    #[serde(rename = "MobileDeviceType")]
    pub mobile_device_type: Option<MobileDeviceType>,

    /// 水平视场角（HorizontalFieldAngle）
    ///
    /// 摄像机的水平视场角，单位为度（°）。
    /// 当为摄像机时可选。
    #[serde(rename = "HorizontalFieldAngle")]
    pub horizontal_field_angle: Option<f64>,

    /// 垂直视场角（VerticalFieldAngle）
    ///
    /// 摄像机的垂直视场角，单位为度（°）。
    /// 当为摄像机时可选。
    #[serde(rename = "VerticalFieldAngle")]
    pub vertical_field_angle: Option<f64>,

    /// 最大可视距离（MaxViewDistance）
    ///
    /// 摄像机的最大可视距离，单位为米（m）。
    /// 当为摄像机时可选。
    #[serde(rename = "MaxViewDistance")]
    pub max_view_distance: Option<f64>,

    /// 基层组织代码（GrassrootsCode）
    ///
    /// 非基层建设时为 "000000"
    /// 摄像机所属基层组织的编码，使用行政区划代码标识。
    /// 当为摄像机且属于基层组织管理范围时可选。
    #[serde(rename = "GrassrootsCode")]
    pub grassroots_code: GrassrootsCode,

    /// 监控点位类型（PointType）
    ///
    /// 标识监控点位的等级分类：
    /// - `1` — 一类监控点位
    /// - `2` — 二类监控点位
    /// - `3` — 三类监控点位
    /// - `9` — 其他
    /// 当为摄像机必选
    #[serde(rename = "PointType")]
    pub point_type: Option<PointType>,

    /// 监控点位常用名称（PointCommonName）
    ///
    /// 监控点位的常用/通俗名称，便于日常识别和管理。
    #[serde(rename = "PointCommonName")]
    pub point_common_name: Option<PointCommonName>,

    /// MAC 地址（MAC）
    ///
    /// 设备的 MAC 地址，格式为 `"XX-XX-XX-XX-XX-XX"`。
    #[serde(rename = "MAC")]
    pub mac: Option<MAC>,

    /// 卡口功能类型列表（FunctionType）
    ///
    /// 标识卡口设备所支持的功能类型：
    /// - `01` — 人脸
    /// - `02` — 人员
    /// - `03` — 机动车
    /// - `04` — 非机动车
    /// - `05` — 物体
    /// - `99` — 其他
    ///
    /// 可多值，用英文半角 "/" 分割，如 `"01/03/99"`。
    /// 当为摄像机时可选。
    #[serde(rename = "FunctionType")]
    pub function_type: Option<FunctionTypes>,

    /// 编码类型（EncodeType）
    ///
    /// 摄像机支持的编码类型，如 `"H.264/H.265/MJPEG"`。
    /// 多个编码类型用英文半角 "/" 分割。
    #[serde(rename = "EncodeType")]
    pub encode_type: Option<EncodeType>,

    /// 安装时间（InstallTime）
    ///
    /// 摄像机的安装日期时间，格式为 ISO 8601（如 `"2024-01-01T00:00:00"`）。
    /// 一类监控点必填
    #[serde(rename = "InstallTime")]
    pub install_time: Option<InstallTime>,

    /// 管理单位（ManagementUnit）
    ///
    /// 摄像机所属的管理单位名称。
    #[serde(rename = "ManagementUnit")]
    pub management_unit: Option<ManagementUnit>,

    /// 联系人信息（ContactInfo）
    ///
    /// 设备/监控点位的联系人信息，如电话号码等。多值用 / 分割
    /// 一类监控点必填
    #[serde(rename = "ContactInfo")]
    pub contact_info: Option<ContactInfo>,

    /// 录像保存天数（RecordSaveDays）
    ///
    /// 一类视频监控点必填，2、3类可选
    #[serde(rename = "RecordSaveDays")]
    pub record_save_days: Option<i32>,

    /// 国民经济行业分类代码（IndustrialClassification）
    ///
    /// 应符合 GB/T 4754—2017 第五章 的规定。
    /// 如 `"I1300"` 表示软件和信息技术服务业。
    #[serde(rename = "IndustrialClassification")]
    pub industrial_classification: Option<IndustrialClassification>,
}

/// 目录项类型
#[derive(Debug, Clone, PartialEq, Default)]
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
    /// 目录项中摄像机的详细参数配置
    pub info: Option<Info>,
}

/// 文件目录项类型（GB/T 28181-2022 附录 A.2.1.10 完整定义）
///
/// 对应文件目录检索应答中的 `<itemFileType>` 元素，描述单个录像文件的信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemFileType {
    /// 目标设备编码（必选）
    #[serde(rename = "DeviceID")]
    pub device_id: DeviceIDType,

    /// 目标设备名称（必选）
    #[serde(rename = "Name")]
    pub name: String,

    /// 文件路径名（可选）
    #[serde(rename = "FilePath")]
    pub file_path: Option<String>,

    /// 录像地址（可选）
    #[serde(rename = "Address")]
    pub address: Option<String>,

    /// 录像开始时间（可选），ISO 8601 格式
    #[serde(rename = "StartTime")]
    pub start_time: Option<String>,

    /// 录像结束时间（可选），ISO 8601 格式
    #[serde(rename = "EndTime")]
    pub end_time: Option<String>,

    /// 保密属性（必选），缺省为 0：0-不涉密，1-涉密
    #[serde(rename = "Secrecy")]
    pub secrecy: u8,

    /// 录像产生类型（可选）：time / alarm / manual (手动)
    #[serde(rename = "Type")]
    pub record_type: Option<String>,

    /// 录像触发者 ID（可选）
    #[serde(rename = "RecorderID")]
    pub recorder_id: Option<String>,

    /// 录像文件大小，单位：Byte（可选）
    #[serde(rename = "FileSize")]
    pub file_size: Option<String>,

    /// 存储录像文件的设备/系统编码（模糊查询时必选）
    #[serde(rename = "RecordLocation")]
    pub record_location: Option<DeviceIDType>,

    /// 码流类型（可选）：0-主码流；1-子码流1；2-子码流2；以此类推
    #[serde(rename = "StreamNumber")]
    pub stream_number: Option<i32>,
}

/// PTZ 精准控制类型（GB/T 28181-2022 附录 A.2.1.11）
///
/// 对应 `<PTZPreciseCtrlType>` 元素，用于设定云台水平角度、垂直角度和变焦倍数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PTZPreciseCtrlType {
    /// 设定云台水平角度（可选），0~360.00 度。
    /// 0 度：绝对 0 度以球机水平光耦为基准，相对 0 度位置以实际设置为准。
    /// 方向：球机竖立安装，从上向下看，顺时针方向增大，逆时针方向减小。
    #[serde(rename = "Pan")]
    pub pan: Option<f64>,

    /// 设定云台垂直角度（可选），一般取值 -30.00~90.00 度。
    /// 0 度：球机竖立安装时，镜头水平位置为 0 度。
    /// 方向：镜头向上转，度数变小；向下转，度数变大。
    #[serde(rename = "Tilt")]
    pub tilt: Option<f64>,

    /// 设定变焦倍数（可选），取值一般大于 1.00。
    /// 若参数在光学变焦最大值内则动作至对应光学变焦倍数，
    /// 超出光学变焦最大值时启动相应数字变焦。
    #[serde(rename = "Zoom")]
    pub zoom: Option<f64>,
}

/// OSD 配置类型（GB/T 28181-2022 附录 A.2.1.12）
///
/// 对应 `<OSDCfgType>` 元素，用于配置前端设备的 OSD 显示参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OSDCfgType {
    /// 配置窗口长度像素值（必选）
    #[serde(rename = "Length")]
    pub length: i32,

    /// 配置窗口宽度像素值（必选）
    #[serde(rename = "Width")]
    pub width: i32,

    /// 时间 X 像素坐标（必选），以播放窗口左上角像素为原点，水平向右为正
    #[serde(rename = "TimeX")]
    pub time_x: i32,

    /// 时间 Y 像素坐标（必选），以播放窗口左上角像素为原点，竖直向下为正
    #[serde(rename = "TimeY")]
    pub time_y: i32,

    /// 显示时间开关（可选），0-关闭；1-打开（默认值 1）
    #[serde(rename = "TimeEnable")]
    pub time_enable: Option<i32>,

    /// 时间显示类型（可选）：0-YYYY-MM-DD HH:MM:SS；1-YYYY年MM月DD日HH:MM:SS
    #[serde(rename = "TimeType")]
    pub time_type: Option<i32>,

    /// 显示文字开关（可选），0-关闭；1-打开（默认值 1）
    #[serde(rename = "TextEnable")]
    pub text_enable: Option<i32>,

    /// 显示文字行数总数（必选）
    #[serde(rename = "SumNum")]
    pub sum_num: i32,

    /// 显示文字列表（可选），最多 8 行
    #[serde(rename = "Item")]
    pub items: Option<Vec<OSDItem>>,
}

/// OSD 文字项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OSDItem {
    /// 文字内容（必选），长度取值范围 0～32
    #[serde(rename = "Text")]
    pub text: String,

    /// 文字 X 坐标（必选）
    #[serde(rename = "X")]
    pub x: i32,

    /// 文字 Y 坐标（必选）
    #[serde(rename = "Y")]
    pub y: i32,
}

/// 视频参数属性配置类型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoParamAttributeCfgType {
    #[serde(rename = "Item", default)]
    pub items: Vec<VideoParamItem>,

    #[serde(rename = "Num")]
    pub num: Option<i32>,
}

/// 视频参数项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoParamItem {
    /// 视频流编号(必选)，0-主码流；1-子码流1；2-子码流2，以此类推
    #[serde(rename = "StreamNumber")]
    pub stream_number: i32,

    /// 视频编码格式(必选)
    #[serde(rename = "VideoFormat")]
    pub video_format: String,

    /// 分辨率(必选)
    #[serde(rename = "Resolution")]
    pub resolution: String,

    /// 帧率(必选)
    #[serde(rename = "FrameRate")]
    pub frame_rate: String,

    /// 码率类型(必选)
    #[serde(rename = "BitRateType")]
    pub bit_rate_type: String,

    /// 视频码率(固定码率时必选)
    #[serde(rename = "VideoBitRate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_bit_rate: Option<String>,
}
// --------------------------------------------------------------------------
/// 移动设备位置类型（GB/T 28181-2022 附录 A.2.1.14）
///
/// 对应 `<itemMobilePositionType>` 元素，描述移动采集设备的位置信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemMobilePositionType {
    /// 目标设备编码（必选）
    #[serde(rename = "DeviceID")]
    pub device_id: DeviceIDType,

    /// 位置采集时间（必选）
    #[serde(rename = "CaptureTime")]
    pub capture_time: String,

    /// 经度（必选），WGS-84 坐标系
    #[serde(rename = "Longitude")]
    pub longitude: f64,

    /// 纬度（必选），WGS-84 坐标系
    #[serde(rename = "Latitude")]
    pub latitude: f64,

    /// 速度（可选），单位：km/h
    #[serde(rename = "Speed")]
    pub speed: Option<f64>,

    /// 方向夹角（可选），取值为当前摄像头方向与正北方的顺时针夹角，取值范围 0~360，单位：度
    #[serde(rename = "Direction")]
    pub direction: Option<f64>,

    /// 海拔高度（可选），单位：米
    #[serde(rename = "Altitude")]
    pub altitude: Option<f64>,

    /// 地面高度（可选），单位：米
    #[serde(rename = "Height")]
    pub height: Option<f64>,
}

/// 录像计划配置类型（GB/T 28181-2022 附录 A.2.1.15）
///
/// 对应 `<videoRecordPlanCfgType>` 元素，用于配置设备的录像计划。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoRecordPlanCfgType {
    /// 是否启用时间计划录像配置（必选）：0-否，1-是
    #[serde(rename = "RecordEnable")]
    pub record_enable: u8,

    /// 每周录像计划总天数（必选）
    #[serde(rename = "RecordScheduleSumNum")]
    pub record_schedule_sum_num: i32,

    /// 一周7天的录像计划（可选），每天最大支持8个时间段配置
    #[serde(rename = "RecordSchedule", default)]
    pub record_schedule: Vec<DayRecordSchedule>,

    /// 码流类型（必选）：0-主码流，1-子码流1，2-子码流2，以此类推
    #[serde(rename = "StreamNumber")]
    pub stream_number: u8,
}

/// 每天的录像计划
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayRecordSchedule {
    /// 周几（必选），取值 1~7，表示周一到周日
    #[serde(rename = "WeekDayNum")]
    pub week_day_num: u8,

    /// 每天录像计划时间段总数（必选）
    #[serde(rename = "TimeSegmentSumNum")]
    pub time_segment_sum_num: i32,

    /// 时间段列表（必选），每天支持最多8个时间段
    #[serde(rename = "TimeSegment", default)]
    pub time_segments: Vec<TimeSegment>,
}

/// 录像时间段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSegment {
    /// 开始时间：时（必选），取值 0~23
    #[serde(rename = "StartHour")]
    pub start_hour: u8,

    /// 开始时间：分（必选），取值 0~59
    #[serde(rename = "StartMin")]
    pub start_min: u8,

    /// 开始时间：秒（必选），取值 0~59
    #[serde(rename = "StartSec")]
    pub start_sec: u8,

    /// 结束时间：时（必选），取值 0~23
    #[serde(rename = "StopHour")]
    pub stop_hour: u8,

    /// 结束时间：分（必选），取值 0~59
    #[serde(rename = "StopMin")]
    pub stop_min: u8,

    /// 结束时间：秒（必选），取值 0~59
    #[serde(rename = "StopSec")]
    pub stop_sec: u8,
}

/// 报警录像配置类型（GB/T 28181-2022 附录 A.2.1.16）
///
/// 对应 `<videoAlarmRecordCfgType>` 元素，用于配置报警触发时的录像行为。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoAlarmRecordCfgType {
    /// 是否启用报警录像配置（必选）：0-否，1-是
    #[serde(rename = "RecordEnable")]
    pub record_enable: u8,

    /// 录像延时时间（可选），报警时间点后的时间，单位：秒
    #[serde(rename = "RecordTime")]
    pub record_time: Option<i32>,

    /// 预录时间（可选），报警时间点前的时间，单位：秒
    #[serde(rename = "PreRecordTime")]
    pub pre_record_time: Option<i32>,

    /// 码流编号（必选）：0-主码流，1-子码流1，2-子码流2，以此类推
    #[serde(rename = "StreamNumber")]
    pub stream_number: u8,
}

/// 视频画面遮挡配置类型（GB/T 28181-2022 附录 A.2.1.17）
///
/// 对应 `<pictureMaskCfgType>` 元素，用于配置视频画面遮挡区域。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PictureMaskCfgType {
    /// 画面遮挡开关（必选）：0-关闭，1-打开
    #[serde(rename = "On")]
    pub on: u8,

    /// 区域总数（必选）
    #[serde(rename = "SumNum")]
    pub sum_num: i32,

    /// 区域列表（可选），最多4个区域
    #[serde(rename = "RegionList")]
    pub region_list: Option<MaskRegionList>,
}

/// 遮挡区域列表
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskRegionList {
    /// 区域项列表
    #[serde(rename = "Item", default)]
    pub items: Vec<MaskRegion>,

    /// 当前区域个数，当无区域时取值为0（必选）
    #[serde(rename = "Num")]
    pub num: i32,
}

/// 单个遮挡区域
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskRegion {
    /// 区域编号（必选），取值范围 1~4
    #[serde(rename = "Seq")]
    pub seq: u8,

    /// 区域坐标（必选），格式如 "20, 30, 50, 60"（左x, 左y, 右x, 右y），单位：像素
    #[serde(rename = "Point")]
    pub point: String,
}

/// 报警上报开关配置类型（GB/T 28181-2022 附录 A.2.1.18）
///
/// 对应 `<alarmReportCfgType>` 元素，用于配置设备报警事件的上报开关。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlarmReportCfgType {
    /// 移动侦测事件上报开关（必选）：0-关闭，1-打开
    #[serde(rename = "MotionDetection")]
    pub motion_detection: u8,

    /// 区域入侵事件上报开关（必选）：0-关闭，1-打开
    #[serde(rename = "FieldDetection")]
    pub field_detection: u8,
}

/// 基本参数配置类型（GB/T 28181-2022 附录 A.2.1.19）
///
/// 对应 `<basicParamCfgType>` 元素，用于配置设备的基本网络和注册参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasicParamCfgType {
    /// 设备名称（可选）
    #[serde(rename = "Name")]
    pub name: Option<String>,

    /// 注册过期时间（可选），单位：秒
    #[serde(rename = "Expiration")]
    pub expiration: Option<i32>,

    /// 心跳间隔时间（可选），单位：秒
    #[serde(rename = "HeartBeatInterval")]
    pub heart_beat_interval: Option<i32>,

    /// 心跳超时次数（可选）
    #[serde(rename = "HeartBeatCount")]
    pub heart_beat_count: Option<i32>,
}

/// 视频参数范围配置类型（GB/T 28181-2022 附录 A.2.1.20）
///
/// 对应 `<videoParamOptCfgType>` 元素，用于描述摄像机支持的视频参数范围。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoParamOptCfgType {
    /// 下载倍速范围（可选），各参数以 "/" 分隔，如 "1/2/4"
    #[serde(rename = "DownloadSpeed")]
    pub download_speed: Option<String>,

    /// 摄像机支持的分辨率（可选），多个值以 "/" 分隔，应符合附录 G 中 SDP f 字段规定
    #[serde(rename = "Resolution")]
    pub resolution: Option<String>,
}

/// SVAC 编码配置类型（GB/T 28181-2022 附录 A.2.1.21）
///
/// 对应 `<SVACEncodeCfgType>` 元素，用于配置 SVAC 视频编码参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SVACEncodeCfgType {
    /// 感兴趣区域参数（可选）
    #[serde(rename = "ROIParam")]
    pub roi_param: Option<ROIParam>,

    /// SVC 参数（可选）
    #[serde(rename = "SVCParam")]
    pub svc_param: Option<SVCParam>,

    /// 监控专用信息参数（仅查询应答可选）
    #[serde(rename = "SurveillanceParam")]
    pub surveillance_param: Option<SurveillanceParam>,

    /// 音频参数（可选）
    #[serde(rename = "AudioParam")]
    pub audio_param: Option<AudioParam>,
}

/// ROI 感兴趣区域参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ROIParam {
    /// 感兴趣区域开关（配置可选，查询应答必选）：0-关闭，1-打开
    #[serde(rename = "ROIFlag")]
    pub roi_flag: Option<u8>,

    /// 感兴趣区域数量（配置可选，查询应答必选），取值范围 0~16
    #[serde(rename = "ROINumber")]
    pub roi_number: Option<u8>,

    /// 感兴趣区域列表（可选），最多16个区域
    #[serde(rename = "Item", default)]
    pub items: Vec<ROIRegion>,
}

/// 单个 ROI 区域
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ROIRegion {
    /// 区域编号（配置可选，查询应答必选），取值范围 1~16
    #[serde(rename = "ROISeq")]
    pub roi_seq: Option<u8>,

    /// 左上角坐标（配置可选，查询应答必选），图像按 32x32 划分后的块序号
    #[serde(rename = "TopLeft")]
    pub top_left: Option<u32>,

    /// 右下角坐标（配置可选，查询应答必选），图像按 32x32 划分后的块序号
    #[serde(rename = "BottomRight")]
    pub bottom_right: Option<u32>,

    /// ROI 区域编码质量等级（配置可选，查询应答必选）：0-一般，1-较好，2-好，3-很好
    #[serde(rename = "ROIQP")]
    pub roi_qp: Option<u8>,
}

/// SVC 参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SVCParam {
    /// 空域编码方式（必选）：0-基本层，1-1级增强，2-2级增强，3-3级增强
    #[serde(rename = "SVCSpaceDomainMode")]
    pub svc_space_domain_mode: u8,

    /// 时域编码方式（必选）：0-基本层，1-1级增强，2-2级增强，3-3级增强
    #[serde(rename = "SVCTimeDomainMode")]
    pub svc_time_domain_mode: u8,

    /// SSVC 增强层与基本层比例值（可选），如 "4:3"、"2:1"、"4:1" 等
    #[serde(rename = "SSVCRatioValue")]
    pub ssvc_ratio_value: Option<String>,

    /// 空域编码能力（仅查询应答必选）：0-不支持，1-1级增强，2-2级增强，3-3级增强
    #[serde(rename = "SVCSpaceSupportMode")]
    pub svc_space_support_mode: Option<u8>,

    /// 时域编码能力（仅查询应答必选）：0-不支持，1-1级增强，2-2级增强，3-3级增强
    #[serde(rename = "SVCTimeSupportMode")]
    pub svc_time_support_mode: Option<u8>,

    /// SSVC 增强层与基本层比例能力（仅查询应答可选），多个值用 "/" 分隔
    #[serde(rename = "SSVCRatioSupportList")]
    pub ssvc_ratio_support_list: Option<String>,
}

/// 监控专用信息参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveillanceParam {
    /// 绝对时间信息开关（必选）：0-关闭，1-打开
    #[serde(rename = "TimeFlag")]
    pub time_flag: Option<u8>,

    /// OSD 信息开关（必选）：0-关闭，1-打开
    #[serde(rename = "OSDFlag")]
    pub osd_flag: Option<u8>,

    /// 智能分析信息开关（必选）：0-关闭，1-打开
    #[serde(rename = "AIFlag")]
    pub ai_flag: Option<u8>,

    /// 地理信息开关（必选）：0-关闭，1-打开
    #[serde(rename = "GISFlag")]
    pub gis_flag: Option<u8>,
}

/// 音频参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioParam {
    /// 声音识别特征参数开关（必选）：0-关闭，1-打开
    #[serde(rename = "AudioRecognitionFlag")]
    pub audio_recognition_flag: u8,
}

/// SVAC 解码配置类型（GB/T 28181-2022 附录 A.2.1.22）
///
/// 对应 `<SVACDecodeCfgType>` 元素，用于配置 SVAC 视频解码参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SVACDecodeCfgType {
    /// SVC 参数（可选）
    #[serde(rename = "SVCParam")]
    pub svc_param: Option<SVCDecodeSVCParam>,

    /// 监控专用信息参数（可选）
    #[serde(rename = "SurveillanceParam")]
    pub surveillance_param: Option<DecodeSurveillanceParam>,
}

/// SVAC 解码的 SVC 参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SVCDecodeSVCParam {
    /// 码流显示模式（配置必选，查询应答可选）：
    /// 0-基本层码流单独显示方式
    /// 1-基本层+1个增强层码流方式
    /// 2-基本层+2个增强层码流方式
    /// 3-基本层+3个增强层码流方式
    #[serde(rename = "SVCSTMMode")]
    pub svc_stm_mode: u8,

    /// 空域编码能力（仅查询应答必选）：0-不支持，1-1级增强，2-2级增强，3-3级增强
    #[serde(rename = "SVCSpaceSupportMode")]
    pub svc_space_support_mode: Option<u8>,

    /// 时域编码能力（仅查询应答必选）：0-不支持，1-1级增强，2-2级增强，3-3级增强
    #[serde(rename = "SVCTimeSupportMode")]
    pub svc_time_support_mode: Option<u8>,
}

/// 解码监控专用信息参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodeSurveillanceParam {
    /// 绝对时间信息显示开关（配置可选，查询应答必选）：0-关闭，1-打开
    #[serde(rename = "TimeShowFlag")]
    pub time_show_flag: Option<u8>,

    /// OSD 信息显示开关（配置可选，查询应答必选）：0-关闭，1-打开
    #[serde(rename = "OSDShowFlag")]
    pub osd_show_flag: Option<u8>,

    /// 智能分析信息显示开关（配置可选，查询应答必选）：0-关闭，1-打开
    #[serde(rename = "AIShowFlag")]
    pub ai_show_flag: Option<u8>,

    /// 地理信息显示开关（配置可选，查询应答必选）：0-关闭，1-打开
    #[serde(rename = "GISShowFlag")]
    pub gis_show_flag: Option<u8>,
}

/// 画面翻转配置类型（GB/T 28181-2022 附录 A.2.1.23）
///
/// 对应 `<frameMirrorCfgType>` 元素，用于配置视频画面镜像翻转方式。
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FrameMirrorCfgType {
    /// 不启用镜像，基准画面
    None = 0,
    /// 水平镜像（左右翻转）
    Horizontal = 1,
    /// 上下镜像（上下翻转）
    Vertical = 2,
    /// 中心镜像（上下左右都翻转）
    Center = 3,
}
impl TryFrom<u8> for FrameMirrorCfgType {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FrameMirrorCfgType::None),
            1 => Ok(FrameMirrorCfgType::Horizontal),
            2 => Ok(FrameMirrorCfgType::Vertical),
            3 => Ok(FrameMirrorCfgType::Center),
            _ => Err("Invalid FrameMirrorCfgType"),
        }
    }
}

/// 图像抓拍配置类型（GB/T 28181-2022 附录 A.2.1.24）
///
/// 对应 `<snapShotCfgType>` 元素，用于配置设备图像抓拍参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotCfgType {
    /// 连拍张数（必选），最多10张，当手动抓拍时取值为1
    #[serde(rename = "SnapNum")]
    pub snap_num: u8,

    /// 单张抓拍间隔时间（可选），单位：秒，最短1秒
    #[serde(rename = "Interval")]
    pub interval: Option<u32>,

    /// 抓拍图像上传路径（必选）
    #[serde(rename = "UploadURL")]
    pub upload_url: String,

    /// 会话 ID（必选），由平台生成，用于关联抓拍的图像与平台请求
    /// 由大小写英文字母、数字、短划线组成，长度 32~128 字节
    #[serde(rename = "SessionID")]
    pub session_id: String,
}

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
