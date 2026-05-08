//! SDP 工具函数（GB28181 规范）
//! xml 编码要求是 GB 18030
//! GB28181 使用标准 SDP，额外规定：
//! - RTP payload type 96 = PS（MPEG-PS 打包）
//! - `y=` 字段携带 SSRC（10 位数字字符串）
//! - `f=` 字段描述视频参数（可选）

use encoding_rs::GB18030;
use quick_xml::Reader;
use std::fmt;
use std::fmt::Write;
use std::str::FromStr;
use tx_di_core::RIE;
/// 判断 IP 字符串是 IPv6 还是 IPv4，返回 `("IP6", addr)` 或 `("IP4", addr)`
///
/// IPv6 地址在 SDP `c=` / `o=` 字段中不需要方括号，直接裸写即可（RFC 4566 §5.7）。
pub fn ip_net_type(ip: &str) -> (&'static str, &str) {
    if ip.contains(':') {
        ("IP6", ip)
    } else {
        ("IP4", ip)
    }
}

/// 会话类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Play,
    Playback,
    Download,
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionType::Play => write!(f, "Play"),
            SessionType::Playback => write!(f, "Playback"),
            SessionType::Download => write!(f, "Download"),
        }
    }
}

impl FromStr for SessionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "play" => Ok(SessionType::Play),
            "playback" => Ok(SessionType::Playback),
            "download" => Ok(SessionType::Download),
            _ => Err(format!("Unknown session type: {}", s)),
        }
    }
}

/// 构建点播/回放/下载 INVITE 的 SDP offer
///
/// 平台向设备发送 INVITE 时携带此 SDP，告知设备将流推到哪里。
///
/// # 参数
///
/// * `local_ip` - 媒体服务器 IP（接收 RTP 流的地址，支持 IPv4 或 IPv6）
/// * `rtp_port` - 接收 RTP 的端口号
/// * `ssrc` - SSRC 标识符（10 位数字字符串，用于区分多路流）
/// * `session_type` - 会话类型（实时点播 Play/ 历史回放 Playback/下载 Download）
/// * `time_range` - 时间范围元组 (开始时间戳, 结束时间戳)，仅 Playback 和 Download 模式必需
/// * `downloadspeed` - 下载速度限制（字节/秒），仅 Download 模式可选
///
/// # 返回值
///
/// 返回格式化后的 SDP 字符串，如果参数校验失败则返回错误
///
/// # 错误
///
/// 当 session_type 为 Playback 或 Download 但未提供 time_range 时返回错误
pub fn build_invite_sdp(
    local_ip: &str,
    rtp_port: u16,
    ssrc: &str,
    session_type: SessionType,
    time_range: Option<(u64, u64)>, // 统一的开始/结束时间戳参数
    downloadspeed: Option<u8>,
) -> RIE<String> {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name: String = session_type.to_string();
    let (addr_type, addr) = ip_net_type(local_ip);

    // 根据业务类型决定时间字段
    let t_field = match session_type {
        SessionType::Play => "t=0 0\r\n".to_string(),
        SessionType::Playback | SessionType::Download => {
            if let Some((start, end)) = time_range {
                format!("t={} {}\r\n", start, end)
            } else {
                return Err("Playback and Download must provide a time range.".into());
            }
        }
    };

    let mut sdp = format!(
        "v=0\r\n\
o=- {session_id} {session_id} IN {addr_type} {addr}\r\n\
s={session_name}\r\n\
c=IN {addr_type} {addr}\r\n\
{t_field}\
t=0 0\r\n\
m=video {rtp_port} RTP/AVP 96\r\n\
a=recvonly\r\n\
a=rtpmap:96 PS/90000\r\n\
y={ssrc}\r\n",
        session_id = session_id,
        addr_type = addr_type,
        addr = addr,
        session_name = session_name,
        t_field = t_field,
        rtp_port = rtp_port,
        ssrc = ssrc
    );
    // 处理下载特有的属性
    if let SessionType::Download = session_type
        && let Some(speed) = downloadspeed
    {
        write!(&mut sdp, "a=downloadspeed:{}\r\n", speed)
            .map_err(|e| format!("SDP format error: {}", e))?;
    }

    Ok(sdp)
}

/// 构建设备回复给平台的 SDP answer（200 OK 中携带）
///
/// # 参数
/// - `local_ip`：设备本地 IP
/// - `rtp_port`：设备推流 RTP 端口
/// - `ssrc`：与 offer 中相同的 SSRC
/// - `device_id`：设备编号，填入 o= 行用户名
/// - `session_type`：会话类型（Play/Playback/Download）
/// - `time_range`：时间范围，Playback/Download 必需
/// - `filesize`: 仅 Download 必需
///
/// # 错误
/// 当 session_type 为 Playback 或 Download 但未提供 time_range 时返回错误
pub fn build_sdp_answer(
    local_ip: &str,
    rtp_port: u16,
    ssrc: &str,
    device_id: &str,
    session_type: SessionType,
    time_range: Option<(u64, u64)>,
    filesize: Option<u64>, // 仅下载场景必需
) -> RIE<String> {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name = match session_type {
        SessionType::Play => "Play",
        SessionType::Playback => "Playback",
        SessionType::Download => "Download",
    };

    let (addrtype, addr) = ip_net_type(local_ip);

    // 时间字段必须与 offer 保持一致
    let t_field = match session_type {
        SessionType::Play => "t=0 0\r\n".to_string(),
        SessionType::Playback | SessionType::Download => {
            if let Some((start, end)) = time_range {
                format!("t={} {}\r\n", start, end)
            } else {
                return Err("Playback and Download must provide a time range.".into());
            }
        }
    };

    let mut sdp = format!(
        "v=0\r\n\
o={device_id} {session_id} {session_id} IN {addrtype} {addr}\r\n\
s={session_name}\r\n\
c=IN {addrtype} {addr}\r\n\
{t_field}\
m=video {rtp_port} RTP/AVP 96\r\n\
a=sendonly\r\n\
a=rtpmap:96 PS/90000\r\n\
y={ssrc}\r\n",
        device_id = device_id,
        session_id = session_id,
        addrtype = addrtype,
        addr = addr,
        session_name = session_name,
        t_field = t_field,
        rtp_port = rtp_port,
        ssrc = ssrc
    );

    if let SessionType::Download = session_type {
        if let Some(size) = filesize {
            write!(&mut sdp, "a=filesize:{}\r\n", size)
                .map_err(|e| format!("SDP format error : {}", e))?;
        } else {
            return Err("Download answer must provide filesize.".into());
        }
    }
    Ok(sdp)
}

/// 从 SDP 中解析 SSRC（`y=` 字段）
pub fn parse_sdp_ssrc(sdp: &str) -> Option<String> {
    // sdp.lines()
    //     .find(|l| l.starts_with("y="))
    //     .map(|l| l[2..].trim().to_string())
    sdp.rfind("y=").and_then(|pos| {
        let line_start = sdp[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line = &sdp[line_start..];
        line.strip_prefix("y=").map(|rest| rest.trim().to_string())
    })
}

pub enum GBMedia {
    Video,
    Audio,
}

impl fmt::Display for GBMedia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GBMedia::Video => write!(f, "video"),
            GBMedia::Audio => write!(f, "audio"),
        }
    }
}

impl FromStr for GBMedia {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "video" => Ok(GBMedia::Video),
            "audio" => Ok(GBMedia::Audio),
            _ => Err(format!("Unknown GBMedia type: {}", s)),
        }
    }
}

/// 从 SDP 中解析指定媒体类型的接收地址和端口
///
/// 支持 GB28181 常见的单媒体流场景，按规范处理 c/m 行。
pub fn parse_sdp_destination(sdp: &str, media: &GBMedia) -> Result<(String, u16), String> {
    let target = media.to_string();

    let mut session_ip: Option<String> = None;
    let mut media_ip: Option<String> = None;
    let mut media_port: Option<String> = None;
    let mut in_target_media = false;

    for line in sdp.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        // 会话级 c= 行（当 media_ip 尚未被 m= 行重写时，会被记录）
        if lower.starts_with("c=in ip4 ") || lower.starts_with("c=in ip6 ") {
            let ip = extract_ip_value(trimmed)?;
            if !in_target_media {
                session_ip = Some(ip);
            } else {
                media_ip = Some(ip);
            }
            continue;
        }

        // m= 新媒体的开始
        if lower.starts_with("m=") {
            // 如果之前正好在目标媒体内，那么现在切换出去就不再处理
            in_target_media = false;

            let body = &trimmed[2..];
            let mut parts = body.split_whitespace();
            let media_type = parts.next().unwrap_or("").to_lowercase();
            if media_type == target {
                in_target_media = true;
                media_port = parts.next().map(|p| {
                    // 处理 "端口号/2" 格式
                    p.split('/').next().unwrap_or(p).to_string()
                });
                // media_ip 先置 None，等待块内的 c=
                media_ip = None;
            }
        }
    }

    // 使用媒体级 IP，若没有则用会话级
    let ip = media_ip
        .or(session_ip)
        .ok_or_else(|| "Missing c= line for media".to_string())?;

    let port_str = media_port.ok_or_else(|| format!("No m={} line", target))?;
    let port = port_str
        .parse::<u16>()
        .map_err(|_| format!("Invalid port: {}", port_str))?;

    Ok((clean_ip(ip), port))
}

/// 从 "c=IN IP4 192.168.1.1" 或 "c=IN IP6 2001:db8::1" 提取 IP 地址
fn extract_ip_value(line: &str) -> Result<String, String> {
    // 安全做法：按空格拆分，取最后一个非空词
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        Ok(parts[2].to_string())
    } else {
        Err(format!("Malformed c= line: {}", line))
    }
}

/// 清理 IPv6 地址中偶尔出现的方括号和区段标识
fn clean_ip(raw: String) -> String {
    raw.trim_matches(|c: char| c == '[' || c == ']')
        .split('%')
        .next()
        .unwrap_or("")
        .to_string()
}

// ── 语音广播/对讲 SDP ─────────────────────────────────────────────────────────

/// 音频编码类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    /// G.711 PCMU (mulaw) 主要在日本和北美用于高保真语音，等同CD音质，带宽占用大（64kbps），音质主要取决于信号源而非编码本身
    PCMU,
    /// G.711 PCMA (alaw) 除欧洲及全球多数地区外，与PCMU同为高保真语音标准，带宽占用64kbps，无需专利费，是VoIP领域兼容性的黄金标准
    PCMA,
    /// 新一代高压缩比音频格式，音质远优于同码率的MP3，能在很宽的码率范围（8kbps~320kbps）保持优质听感，且支持多声道
    AAC,
    /// 7~14kHz宽带高清语音编码，以32/48kbps的较低码率实现自然清晰的音质，比普通电话频带更宽，但算法延迟略高（40ms）
    G_722_1,
    /// 极低带宽双速率语音编码（5.3/6.3 kbps），码率极低但音质一般且有专利，常用于最早期的窄带视频会议
    G_723_1,
    /// 经典的8kbps低带宽高品质通话编码，用1/8的带宽实现了接近G.711的通话质量，低延迟，但因为专利和复杂度正逐渐被Opus替代
    G_729,
    /// 中国自主可控的智能监控专属标准。在编码音频时能内嵌绝对时间、特征参数等监控信息，支持加密防篡改和感兴趣区域编码，主要用于公安等高安全等级项目。
    SVAC,
}

impl AudioCodec {
    /// 返回 RTPMAP 所需的 (payload_type, encoding_name/clock_rate)
    ///
    /// 说明：
    /// - PCMU / PCMA / G.723.1 / G.729 为静态 payload type（RFC 3551）
    /// - AAC / G.722.1 / SVAC 为动态 payload type，此处采用 GB28181 常见约定
    /// - 实际使用时，动态类型可通过 SDP 协商，不与固定值冲突即可
    pub fn rtpmap(&self) -> (u8, &'static str) {
        match self {
            AudioCodec::PCMU    => (0,   "PCMU/8000"),
            AudioCodec::PCMA    => (8,   "PCMA/8000"),
            AudioCodec::AAC     => (102, "AAC/8000"),      // 常见动态 PT=102，采样率 8kHz
            AudioCodec::G_722_1 => (104, "G7221/16000"),   // 常见动态 PT=104，采样率 16kHz
            AudioCodec::G_723_1 => (4,   "G723/8000"),     // 静态 PT=4，采样率 8kHz
            AudioCodec::G_729   => (18,  "G729/8000"),     // 静态 PT=18，采样率 8kHz
            AudioCodec::SVAC    => (100, "SVAC/8000"),     // 常见动态 PT=100，采样率 8kHz
        }
    }
}

/// 音频会话信息
#[derive(Debug, Clone)]
pub struct AudioSessionInfo {
    /// 设备 IP
    pub device_ip: String,
    /// 设备 RTP 端口
    pub device_port: u16,
    /// SSRC
    pub ssrc: String,
    /// 音频编码
    pub codec: AudioCodec,
}

/// 构建对讲 INVITE 的 SDP offer（平台发送音频给设备）
///
/// GB28181-2022 §9.12：语音对讲时，平台在 INVITE 中包含音频流
/// `a=sendonly` 表示平台向设备发送音频
pub fn build_audio_invite_sdp(
    local_ip: &str,
    video_port: u16,
    audio_port: u16,
    codec: AudioCodec,
    ssrc: &str,
) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (audio_pt, audio_rtpmap) = codec.rtpmap();
    let (addrtype, addr) = ip_net_type(local_ip);

    format!(
        "v=0\r\n\
o=- {session_id} {session_id} IN {addrtype} {addr}\r\n\
s=Play\r\n\
c=IN {addrtype} {addr}\r\n\
t=0 0\r\n\
m=video {video_port} RTP/AVP 96\r\n\
a=sendonly\r\n\
a=rtpmap:96 PS/90000\r\n\
y={ssrc}\r\n\
m=audio {audio_port} RTP/AVP {audio_pt}\r\n\
a=sendonly\r\n\
a=rtpmap:{audio_pt} {audio_rtpmap}\r\n\
y={ssrc}\r\n",
        session_id = session_id,
        addrtype = addrtype,
        addr = addr,
        video_port = video_port,
        audio_port = audio_port,
        audio_pt = audio_pt,
        audio_rtpmap = audio_rtpmap,
        ssrc = ssrc
    )
}

/// 构建广播邀请响应 SDP（设备推流给平台）
///
/// GB28181-2022 §9.12：设备收到广播邀请后，响应此 SDP 并开始推流
pub fn build_broadcast_answer_sdp(
    local_ip: &str,
    audio_port: u16,
    codec: AudioCodec,
    ssrc: &str,
) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (audio_pt, audio_rtpmap) = codec.rtpmap();
    let (addrtype, addr) = ip_net_type(local_ip);

    format!(
        "v=0\r\n\
o=- {session_id} {session_id} IN {addrtype} {addr}\r\n\
s=Broadcast\r\n\
c=IN {addrtype} {addr}\r\n\
t=0 0\r\n\
m=audio {audio_port} RTP/AVP {audio_pt}\r\n\
a=recvonly\r\n\
a=rtpmap:{audio_pt} {audio_rtpmap}\r\n\
y={ssrc}\r\n",
        session_id = session_id,
        addrtype = addrtype,
        addr = addr,
        audio_port = audio_port,
        audio_pt = audio_pt,
        audio_rtpmap = audio_rtpmap,
        ssrc = ssrc
    )
}

/// 从对讲 SDP 中解析音频信息
///
/// 同时支持 IPv4 和 IPv6 的 `c=` 行。
pub fn parse_audio_sdp(sdp: &str) -> Option<AudioSessionInfo> {
    let mut device_ip = String::new();
    let mut device_port = 0u16;
    let mut ssrc = String::new();
    let mut codec = AudioCodec::Pcmu;

    for line in sdp.lines() {
        if let Some(rest) = line
            .strip_prefix("c=IN IP4 ")
            .or_else(|| line.strip_prefix("c=IN IP6 "))
        {
            device_ip = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("m=audio ") {
            if let Some(p) = rest.split_whitespace().next() {
                device_port = p.parse().unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("y=") {
            ssrc = rest.trim().to_string();
        } else if line.contains("PCMA") {
            codec = AudioCodec::Pcma;
        } else if line.contains("G726") {
            codec = AudioCodec::G726;
        }
        // PCMU 是默认值，不需要显式处理
    }

    if device_port > 0 {
        Some(AudioSessionInfo {
            device_ip,
            device_port,
            ssrc,
            codec,
        })
    } else {
        None
    }
}

// ── 抓拍 SDP todo 抓拍 2022 有变化 ─────────────────────────────────────────────────────────────────

/// 抓拍会话信息 2016
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    /// 抓拍图片的 URL（从 SDP `u=` 字段解析，含冒号后的版本号）
    pub image_url: String,
    /// 抓拍描述（`u=` 中冒号前的 URL 部分）
    pub description: String,
}

/// 构建抓拍 INVITE 的 SDP offer
///
/// GB28181-2022 §9.14：平台向设备发起抓拍请求
/// `s=SnapShot`，`u=` 携带描述，`y=` 携带 SSRC
pub fn build_snapshot_sdp(local_ip: &str, sn: u32) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let ssrc = format!("{:010}", sn);
    let (addrtype, addr) = ip_net_type(local_ip);

    format!(
        "v=0\r\n\
o=- {session_id} {session_id} IN {addrtype} {addr}\r\n\
s=SnapShot\r\n\
u=http://localhost:8080/snapshot?sn={sn}:0\r\n\
c=IN {addrtype} {addr}\r\n\
t=0 0\r\n\
m=image 0 RTP/AVP 98\r\n\
a=rtpmap:98 jpeg/90000\r\n\
y={ssrc}\r\n",
        session_id = session_id,
        addrtype = addrtype,
        addr = addr,
        sn = sn,
        ssrc = ssrc
    )
}

/// 从 SDP 中解析抓拍图片 URL
///
/// `u=` 字段格式为 `<uri>:<version>`，例如 `http://...?sn=1:0`。
/// - `image_url`：完整 `u=` 值（含版本号）
/// - `description`：冒号前的纯 URI 部分
pub fn parse_snapshot_sdp(sdp: &str) -> SnapshotInfo {
    let raw = sdp
        .lines()
        .find(|l| l.starts_with("u="))
        .map(|l| l[2..].trim())
        .unwrap_or_default();

    let image_url = raw.to_string();
    // u= 格式：<uri>:<version>，取最后一个冒号之前的部分作为纯 URI
    let description = raw.rfind(':').map(|i| &raw[..i]).unwrap_or(raw).to_string();

    SnapshotInfo {
        image_url,
        description,
    }
}

/// 符合 GB/T 28181-2022 的抓拍信息结构体
#[derive(Debug, Clone, Default)]
pub struct SnapshotInfo2022 {
    pub device_id: String,
    pub session_id: String,
    pub upload_url: String,
    pub interval: u32,
    pub count: u32,
}

/// 从 GB 18030 编码的 XML 字节流中解析抓拍配置信息
pub fn parse_snapshot_info_from_xml(xml_bytes: &[u8]) -> Result<SnapshotInfo2022, String> {
    // 第一步：GB18030 解码为 UTF-8
    let (utf8_string, _, had_errors) = GB18030.decode(xml_bytes);
    if had_errors {
        // 严格模式下可返回错误，这里选择容忍并继续
        eprintln!("警告：GB18030 解码出现替换字符");
    }

    let xml_str = utf8_string; // Cow<str>，可解引用为 &str

    // 第二步：使用 quick_xml 解析
    let mut reader = Reader::from_str(&xml_str);

    let mut info = SnapshotInfo2022::default();
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
            }
            Ok(quick_xml::events::Event::Text(e)) => {
                let text = String::from_utf8_lossy(&e).into_owned();
                match current_tag.as_str() {
                    "DeviceID" => info.device_id = text,
                    "SessionID" => info.session_id = text,
                    "UploadURL" => info.upload_url = text,
                    "Interval" => info.interval = text.parse().unwrap_or(0),
                    "Count" => info.count = text.parse().unwrap_or(1),
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(format!("XML解析错误: {}", e)),
            _ => {}
        }
    }

    if info.session_id.is_empty() {
        return Err("XML中缺少会话ID (SessionID)".to_string());
    }

    Ok(info)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_invite_sdp ─────────────────────────────────────────────────────

    #[test]
    fn invite_sdp_realtime_play() {
        let sdp = build_invite_sdp("192.168.1.100", 10000, "0000000001", true);
        assert!(sdp.contains("s=Play"));
        assert!(sdp.contains("c=IN IP4 192.168.1.100"));
        assert!(sdp.contains("m=video 10000 RTP/AVP 96"));
        assert!(sdp.contains("a=recvonly"));
        assert!(sdp.contains("a=rtpmap:96 PS/90000"));
        assert!(sdp.contains("y=0000000001"));
    }

    #[test]
    fn invite_sdp_playback() {
        let sdp = build_invite_sdp("10.0.0.1", 20000, "1234567890", false);
        assert!(sdp.contains("s=Playback"));
        assert!(sdp.contains("m=video 20000"));
        assert!(sdp.contains("y=1234567890"));
    }

    #[test]
    fn invite_sdp_ipv6() {
        let sdp = build_invite_sdp("::1", 30000, "0000000001", true);
        assert!(sdp.contains("c=IN IP6 ::1"));
        assert!(sdp.contains("o=-"));
    }

    // ── build_sdp_answer ─────────────────────────────────────────────────────

    #[test]
    fn sdp_answer_realtime() {
        let sdp = build_sdp_answer(
            "192.168.1.50",
            10000,
            "0000000001",
            "34020000001320000001",
            true,
        );
        assert!(sdp.contains("s=Play"));
        assert!(sdp.contains("a=sendonly"));
        assert!(sdp.contains("o=34020000001320000001"));
        assert!(sdp.contains("c=IN IP4 192.168.1.50"));
    }

    #[test]
    fn sdp_answer_playback() {
        let sdp = build_sdp_answer(
            "192.168.1.50",
            10000,
            "0000000001",
            "34020000001320000001",
            false,
        );
        assert!(sdp.contains("s=Playback"));
    }

    #[test]
    fn sdp_answer_ipv6() {
        let sdp = build_sdp_answer("fe80::1", 10000, "0000000001", "device1", true);
        assert!(sdp.contains("c=IN IP6 fe80::1"));
    }

    // ── parse_sdp_ssrc ───────────────────────────────────────────────────────

    #[test]
    fn parse_ssrc_present() {
        let sdp = "v=0\r\ny=1234567890\r\n";
        assert_eq!(parse_sdp_ssrc(sdp), Some("1234567890".to_string()));
    }

    #[test]
    fn parse_ssrc_absent() {
        let sdp = "v=0\r\ns=Play\r\n";
        assert_eq!(parse_sdp_ssrc(sdp), None);
    }

    // ── parse_sdp_destination ────────────────────────────────────────────────

    #[test]
    fn parse_destination_ipv4() {
        let sdp = "c=IN IP4 192.168.1.100\r\nm=video 10000 RTP/AVP 96\r\n";
        let (ip, port) = parse_sdp_destination(sdp);
        assert_eq!(ip, "192.168.1.100");
        assert_eq!(port, 10000);
    }

    #[test]
    fn parse_destination_ipv6() {
        let sdp = "c=IN IP6 ::1\r\nm=video 20000 RTP/AVP 96\r\n";
        let (ip, port) = parse_sdp_destination(sdp);
        assert_eq!(ip, "::1");
        assert_eq!(port, 20000);
    }

    #[test]
    fn parse_destination_empty_defaults_to_any() {
        let sdp = "v=0\r\nm=video 5000 RTP/AVP 96\r\n";
        let (ip, port) = parse_sdp_destination(sdp);
        assert_eq!(ip, "0.0.0.0");
        assert_eq!(port, 5000);
    }

    // ── build_audio_invite_sdp ───────────────────────────────────────────────

    #[test]
    fn audio_invite_sdp_has_video_and_audio() {
        let sdp =
            build_audio_invite_sdp("192.168.1.1", 10000, 20000, AudioCodec::Pcmu, "0000000001");
        assert!(sdp.contains("m=video 10000"));
        assert!(sdp.contains("m=audio 20000"));
        assert!(sdp.contains("a=rtpmap:0 PCMU/8000"));
        assert!(sdp.contains("a=sendonly"));
    }

    #[test]
    fn audio_invite_sdp_pcma_codec() {
        let sdp = build_audio_invite_sdp("10.0.0.1", 10000, 20000, AudioCodec::Pcma, "0000000001");
        assert!(sdp.contains("a=rtpmap:8 PCMA/8000"));
    }

    #[test]
    fn audio_invite_sdp_g726_codec() {
        let sdp = build_audio_invite_sdp("10.0.0.1", 10000, 20000, AudioCodec::G726, "0000000001");
        assert!(sdp.contains("a=rtpmap:96 G726-16/8000"));
    }

    // ── build_broadcast_answer_sdp ───────────────────────────────────────────

    #[test]
    fn broadcast_answer_sdp_has_recvonly() {
        let sdp = build_broadcast_answer_sdp("192.168.1.1", 30000, AudioCodec::Pcmu, "0000000001");
        assert!(sdp.contains("s=Broadcast"));
        assert!(sdp.contains("m=audio 30000"));
        assert!(sdp.contains("a=recvonly"));
    }

    // ── parse_audio_sdp ──────────────────────────────────────────────────────

    #[test]
    fn parse_audio_sdp_pcma() {
        let sdp = "c=IN IP4 192.168.1.50\r\nm=audio 30000 RTP/AVP 8\r\na=rtpmap:8 PCMA/8000\r\ny=0000000001\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.device_ip, "192.168.1.50");
        assert_eq!(info.device_port, 30000);
        assert_eq!(info.ssrc, "0000000001");
        assert_eq!(info.codec, AudioCodec::Pcma);
    }

    #[test]
    fn parse_audio_sdp_pcmu_default() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 0\r\ny=0000000002\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::Pcmu); // 默认
    }

    #[test]
    fn parse_audio_sdp_ipv6() {
        let sdp = "c=IN IP6 fe80::1\r\nm=audio 40000 RTP/AVP 0\r\ny=0000000003\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.device_ip, "fe80::1");
    }

    #[test]
    fn parse_audio_sdp_no_audio_returns_none() {
        let sdp = "c=IN IP4 192.168.1.1\r\n";
        assert!(parse_audio_sdp(sdp).is_none());
    }

    // ── build_snapshot_sdp ───────────────────────────────────────────────────

    #[test]
    fn snapshot_sdp_structure() {
        let sdp = build_snapshot_sdp("192.168.1.1", 42);
        assert!(sdp.contains("s=SnapShot"));
        assert!(sdp.contains("m=image 0 RTP/AVP 98"));
        assert!(sdp.contains("a=rtpmap:98 jpeg/90000"));
        assert!(sdp.contains("y=0000000042"));
    }

    // ── parse_snapshot_sdp ───────────────────────────────────────────────────

    #[test]
    fn parse_snapshot_sdp_with_version() {
        let sdp = "v=0\r\nu=http://192.168.1.1:8080/snapshot?sn=42:0\r\n";
        let info = parse_snapshot_sdp(sdp);
        assert_eq!(info.image_url, "http://192.168.1.1:8080/snapshot?sn=42:0");
        assert_eq!(info.description, "http://192.168.1.1:8080/snapshot?sn=42");
    }

    #[test]
    fn parse_snapshot_sdp_no_u_field() {
        let sdp = "v=0\r\ns=SnapShot\r\n";
        let info = parse_snapshot_sdp(sdp);
        assert!(info.image_url.is_empty());
    }

    // ── AudioCodec rtpmap ────────────────────────────────────────────────────

    #[test]
    fn audio_codec_rtpmap() {
        assert_eq!(AudioCodec::Pcmu.rtpmap(), (0, "PCMU/8000"));
        assert_eq!(AudioCodec::Pcma.rtpmap(), (8, "PCMA/8000"));
        assert_eq!(AudioCodec::G726.rtpmap(), (96, "G726-16/8000"));
    }
}
