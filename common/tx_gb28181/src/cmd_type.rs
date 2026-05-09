//! GB28181 MESSAGE CmdType 枚举
//!
//! 对应 GB28181-2022 标准中所有 MESSAGE 消息的 CmdType 字段值。
//! 支持大小写不敏感解析（兼容不同厂商实现）。

/// GB28181 MESSAGE CmdType 枚举
///
/// 16 个
///
/// 覆盖 GB28181-2016 / 2022 标准中定义的所有 CmdType 值，
/// 包括 2022 版新增的 CruiseTrack、PtzPreciseStatus、GuardInfo 等。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Gb28181CmdType {
    /// 心跳保活
    Keepalive,
    /// 目录查询响应
    Catalog,
    /// 设备信息查询响应
    DeviceInfo,
    /// 设备状态查询响应
    DeviceStatus,
    /// 录像信息查询响应
    RecordInfo,
    /// 报警通知
    Alarm,
    /// 媒体流状态通知（设备推流结束等）
    MediaStatus,
    /// 移动设备位置上报
    MobilePosition,
    /// 设备配置下载响应
    ConfigDownload,
    /// 预置位查询响应
    PresetList,
    /// 巡航轨迹列表查询响应
    CruiseList,
    /// 预置位查询响应（非标准 CmdType，部分设备使用 `PresetQuery` 替代 `PresetList`）
    PresetQuery,
    /// 巡航轨迹详情响应（GB28181-2022 新增）
    CruiseTrack,
    /// PTZ 精准状态响应（GB28181-2022 新增）
    PtzPreciseStatus,
    /// 看守位信息查询响应（GB28181-2022 新增）
    GuardInfo,
    /// 语音广播消息（Invite / TearDown）
    Broadcast,
}

impl std::fmt::Display for Gb28181CmdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keepalive       => write!(f, "Keepalive"),
            Self::Catalog         => write!(f, "Catalog"),
            Self::DeviceInfo      => write!(f, "DeviceInfo"),
            Self::DeviceStatus    => write!(f, "DeviceStatus"),
            Self::RecordInfo      => write!(f, "RecordInfo"),
            Self::Alarm           => write!(f, "Alarm"),
            Self::MediaStatus     => write!(f, "MediaStatus"),
            Self::MobilePosition  => write!(f, "MobilePosition"),
            Self::ConfigDownload  => write!(f, "ConfigDownload"),
            Self::PresetList      => write!(f, "PresetList"),
            Self::CruiseList      => write!(f, "CruiseList"),
            Self::PresetQuery     => write!(f, "PresetQuery"),
            Self::CruiseTrack     => write!(f, "CruiseTrack"),
            Self::PtzPreciseStatus => write!(f, "PtzPreciseStatus"),
            Self::GuardInfo       => write!(f, "GuardInfo"),
            Self::Broadcast       => write!(f, "Broadcast"),
        }
    }
}

impl std::str::FromStr for Gb28181CmdType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 快速路径：精确匹配（绝大多数设备使用标准大小写）
        let cmd = match s {
            "Keepalive"        => return Ok(Self::Keepalive),
            "Catalog"          => return Ok(Self::Catalog),
            "DeviceInfo"       => return Ok(Self::DeviceInfo),
            "DeviceStatus"     => return Ok(Self::DeviceStatus),
            "RecordInfo"       => return Ok(Self::RecordInfo),
            "Alarm"            => return Ok(Self::Alarm),
            "MediaStatus"      => return Ok(Self::MediaStatus),
            "MobilePosition"   => return Ok(Self::MobilePosition),
            "ConfigDownload"   => return Ok(Self::ConfigDownload),
            "PresetList"       => return Ok(Self::PresetList),
            "CruiseList"       => return Ok(Self::CruiseList),
            "PresetQuery"      => return Ok(Self::PresetQuery),
            "CruiseTrack"      => return Ok(Self::CruiseTrack),
            "PtzPreciseStatus" => return Ok(Self::PtzPreciseStatus),
            "GuardInfo"        => return Ok(Self::GuardInfo),
            "Broadcast"        => return Ok(Self::Broadcast),
            _ => s,
        };
        // 大小写不敏感兜底（兼容部分厂商全小写/全大写实现）
        let lower = cmd.to_ascii_lowercase();
        match lower.as_str() {
            "keepalive"        => Ok(Self::Keepalive),
            "catalog"          => Ok(Self::Catalog),
            "deviceinfo"       => Ok(Self::DeviceInfo),
            "devicestatus"     => Ok(Self::DeviceStatus),
            "recordinfo"       => Ok(Self::RecordInfo),
            "alarm"            => Ok(Self::Alarm),
            "mediastatus"      => Ok(Self::MediaStatus),
            "mobileposition"   => Ok(Self::MobilePosition),
            "configdownload"   => Ok(Self::ConfigDownload),
            "presetlist"       => Ok(Self::PresetList),
            "cruiselist"       => Ok(Self::CruiseList),
            "presetquery"      => Ok(Self::PresetQuery),
            "cruisetrack"      => Ok(Self::CruiseTrack),
            "ptzprecisestatus" => Ok(Self::PtzPreciseStatus),
            "guardinfo"        => Ok(Self::GuardInfo),
            "broadcast"        => Ok(Self::Broadcast),
            _                  => Err(format!("未知的 GB28181 指令类型: {cmd}")),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exact_case() {
        assert_eq!("Keepalive".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::Keepalive);
        assert_eq!("Catalog".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::Catalog);
        assert_eq!("Alarm".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::Alarm);
        assert_eq!("Broadcast".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::Broadcast);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!("keepalive".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::Keepalive);
        assert_eq!("CATALOG".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::Catalog);
        assert_eq!("cruiseTrack".parse::<Gb28181CmdType>().unwrap(), Gb28181CmdType::CruiseTrack);
    }

    #[test]
    fn parse_unknown() {
        assert!("UnknownCmd".parse::<Gb28181CmdType>().is_err());
    }

    #[test]
    fn display_roundtrip() {
        let cmd = Gb28181CmdType::PtzPreciseStatus;
        let s = format!("{}", cmd);
        assert_eq!(s.parse::<Gb28181CmdType>().unwrap(), cmd);
    }
}
