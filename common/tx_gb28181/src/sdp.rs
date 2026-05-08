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
/// * `local_ip`      - 媒体服务器 IP（接收 RTP 流的地址，支持 IPv4 或 IPv6）
/// * `rtp_port`      - 接收 RTP 的端口号
/// * `ssrc`          - SSRC 标识符（10 位数字字符串，用于区分多路流）
/// * `session_type`  - 会话类型（实时点播 Play / 历史回放 Playback / 下载 Download）
/// * `time_range`    - 时间范围元组 (开始时间戳, 结束时间戳)，Playback / Download 必需
/// * `downloadspeed` - 下载速度倍率（整数），仅 Download 模式可选
///
/// # 错误
///
/// 当 `session_type` 为 `Playback` 或 `Download` 但未提供 `time_range` 时返回错误。
pub fn build_invite_sdp(
    local_ip: &str,
    rtp_port: u16,
    ssrc: &str,
    session_type: SessionType,
    time_range: Option<(u64, u64)>,
    downloadspeed: Option<u8>,
) -> RIE<String> {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name = session_type.to_string();
    let (addr_type, addr) = ip_net_type(local_ip);

    // 根据会话类型决定 t= 字段（GB28181-2016 §A.2.1）
    let t_field = match session_type {
        SessionType::Play => "t=0 0\r\n".to_string(),
        SessionType::Playback | SessionType::Download => {
            if let Some((start, end)) = time_range {
                format!("t={} {}\r\n", start, end)
            } else {
                return Err("Playback 和 Download 会话必须提供时间范围".into());
            }
        }
    };

    let mut sdp = format!(
        "v=0\r\n\
o=- {session_id} {session_id} IN {addr_type} {addr}\r\n\
s={session_name}\r\n\
c=IN {addr_type} {addr}\r\n\
{t_field}\
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

    // Download 特有属性（GB28181-2016 §A.2.3）
    if let SessionType::Download = session_type
        && let Some(speed) = downloadspeed
    {
        write!(&mut sdp, "a=downloadspeed:{}\r\n", speed)
            .map_err(|e| format!("SDP 格式化错误: {}", e))?;
    }

    Ok(sdp)
}

/// 构建设备回复给平台的 SDP answer（200 OK 中携带）
///
/// # 参数
/// - `local_ip`     ：设备本地 IP（支持 IPv4 / IPv6）
/// - `rtp_port`     ：设备推流 RTP 端口
/// - `ssrc`         ：与 offer 中相同的 SSRC
/// - `device_id`    ：设备编号，填入 `o=` 行用户名字段
/// - `session_type` ：会话类型（Play / Playback / Download）
/// - `time_range`   ：时间范围，Playback / Download 必需
/// - `filesize`     ：仅 Download 必需，单位字节
///
/// # 错误
/// 当 `session_type` 为 Playback 或 Download 但未提供 `time_range` 时返回错误；
/// 当 `session_type` 为 Download 但未提供 `filesize` 时返回错误。
pub fn build_sdp_answer(
    local_ip: &str,
    rtp_port: u16,
    ssrc: &str,
    device_id: &str,
    session_type: SessionType,
    time_range: Option<(u64, u64)>,
    filesize: Option<u64>,
) -> RIE<String> {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name = session_type.to_string();
    let (addrtype, addr) = ip_net_type(local_ip);

    // 时间字段必须与 offer 保持一致（GB28181-2016 §A.2.2）
    let t_field = match session_type {
        SessionType::Play => "t=0 0\r\n".to_string(),
        SessionType::Playback | SessionType::Download => {
            if let Some((start, end)) = time_range {
                format!("t={} {}\r\n", start, end)
            } else {
                return Err("Playback 和 Download 会话必须提供时间范围".into());
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

    // Download answer 必须附加 filesize（GB28181-2016 §A.2.3）
    if let SessionType::Download = session_type {
        if let Some(size) = filesize {
            write!(&mut sdp, "a=filesize:{}\r\n", size)
                .map_err(|e| format!("SDP 格式化错误: {}", e))?;
        } else {
            return Err("Download answer 必须提供 filesize".into());
        }
    }

    Ok(sdp)
}

/// 从 SDP 中解析 SSRC（`y=` 字段，取最后一个匹配）
pub fn parse_sdp_ssrc(sdp: &str) -> Option<String> {
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

        // 会话级 / 媒体级 c= 行
        if lower.starts_with("c=in ip4 ") || lower.starts_with("c=in ip6 ") {
            let ip = extract_ip_value(trimmed)?;
            if !in_target_media {
                session_ip = Some(ip);
            } else {
                media_ip = Some(ip);
            }
            continue;
        }

        // m= 行：每次遇到都重置 in_target_media
        if lower.starts_with("m=") {
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
                // 媒体级 IP 先清空，等块内的 c= 行覆盖
                media_ip = None;
            }
        }
    }

    // 媒体级 IP 优先；如无则用会话级
    let ip = media_ip
        .or(session_ip)
        .ok_or_else(|| "SDP 缺少 c= 行".to_string())?;
    let port_str = media_port.ok_or_else(|| format!("SDP 缺少 m={} 行", target))?;
    let port = port_str
        .parse::<u16>()
        .map_err(|_| format!("无效端口: {}", port_str))?;

    Ok((clean_ip(ip), port))
}

/// 从 "c=IN IP4 192.168.1.1" 或 "c=IN IP6 2001:db8::1" 提取 IP 地址
fn extract_ip_value(line: &str) -> Result<String, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        Ok(parts[2].to_string())
    } else {
        Err(format!("c= 行格式错误: {}", line))
    }
}

/// 清理 IPv6 地址中偶尔出现的方括号和区段标识（%eth0 等）
fn clean_ip(raw: String) -> String {
    raw.trim_matches(|c: char| c == '[' || c == ']')
        .split('%')
        .next()
        .unwrap_or("")
        .to_string()
}

// ── 语音广播/对讲 SDP ─────────────────────────────────────────────────────────

/// 音频编码类型
///
/// 覆盖 GB28181-2016 / 2022 规范中明确支持的编码格式。
/// payload type 对应 RFC 3551 静态值或 GB28181 惯例动态值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    /// G.711 PCMU (μ-law) 静态 PT=0；8 kHz，64 kbps；主要用于北美/日本
    PCMU,
    /// G.711 PCMA (A-law) 静态 PT=8；8 kHz，64 kbps；欧洲及全球 VoIP 黄金标准
    PCMA,
    /// AAC 动态 PT=102（GB28181 惯例）；支持 8 kbps~320 kbps，多声道
    AAC,
    /// G.722.1 宽带语音动态 PT=104；16 kHz，32/48 kbps；延迟约 40 ms
    G7221,
    /// G.723.1 双速率窄带语音静态 PT=4；8 kHz，5.3/6.3 kbps
    G7231,
    /// G.729 低带宽语音静态 PT=18；8 kHz，8 kbps
    G729,
    /// SVAC（GB/T 26197）动态 PT=100（GB28181 惯例）；中国安防专用，支持内嵌监控信息
    SVAC,
}

impl AudioCodec {
    /// 返回 `(payload_type, "encoding_name/clock_rate")` 二元组，用于 `a=rtpmap` 行
    ///
    /// - 静态 PT：PCMU(0)、PCMA(8)、G.723.1(4)、G.729(18) — 符合 RFC 3551
    /// - 动态 PT：AAC(102)、G.722.1(104)、SVAC(100) — 采用 GB28181 行业惯例值
    pub fn rtpmap(&self) -> (u8, &'static str) {
        match self {
            AudioCodec::PCMU => (0, "PCMU/8000"),
            AudioCodec::PCMA => (8, "PCMA/8000"),
            AudioCodec::AAC => (102, "AAC/8000"),
            AudioCodec::G7221 => (104, "G7221/16000"),
            AudioCodec::G7231 => (4, "G723/8000"),
            AudioCodec::G729 => (18, "G729/8000"),
            AudioCodec::SVAC => (100, "SVAC/8000"),
        }
    }
}

/// 音频会话信息（对讲/广播场景）
#[derive(Debug, Clone)]
pub struct AudioSessionInfo {
    /// 对端（设备）IP
    pub device_ip: String,
    /// 对端 RTP 端口
    pub device_port: u16,
    /// SSRC（`y=` 字段）
    pub ssrc: String,
    /// 协商到的音频编码
    pub codec: AudioCodec,
}

/// 构建语音对讲 INVITE 的 SDP offer（平台 → 设备）
///
/// GB28181-2022 §9.12：平台在 INVITE 中同时携带视频流和音频流。
/// `a=sendonly` 表示平台向设备发送音频（设备只收，不发）。
///
/// # SDP 结构
/// ```text
/// v=0
/// o=- <session_id> <session_id> IN IP4 <local_ip>
/// s=Play
/// c=IN IP4 <local_ip>
/// t=0 0
/// m=video <video_port> RTP/AVP 96
/// a=sendonly
/// a=rtpmap:96 PS/90000
/// y=<ssrc>
/// m=audio <audio_port> RTP/AVP <pt>
/// a=sendonly
/// a=rtpmap:<pt> <encoding/clock_rate>
/// y=<ssrc>
/// ```
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

/// 构建广播邀请响应 SDP（设备 → 平台，仅音频流）
///
/// GB28181-2022 §9.12：设备收到广播 INVITE 后，以此 SDP 作为 200 OK body。
/// `a=recvonly` 表示设备只接收音频（平台负责发送）。
///
/// # SDP 结构
/// ```text
/// v=0
/// o=- <session_id> <session_id> IN IP4 <local_ip>
/// s=Broadcast
/// c=IN IP4 <local_ip>
/// t=0 0
/// m=audio <audio_port> RTP/AVP <pt>
/// a=recvonly
/// a=rtpmap:<pt> <encoding/clock_rate>
/// y=<ssrc>
/// ```
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

/// 从对讲 / 广播 SDP 中解析音频会话信息
///
/// 同时支持 IPv4 / IPv6 的 `c=` 行。  
/// 编解码识别优先级（`a=rtpmap` 行）：PCMA > SVAC > AAC > G.722.1 > G.723.1 > G.729 > PCMU（默认）。
pub fn parse_audio_sdp(sdp: &str) -> Option<AudioSessionInfo> {
    let mut device_ip = String::new();
    let mut device_port = 0u16;
    let mut ssrc = String::new();
    let mut codec = AudioCodec::PCMU; // 默认 PCMU（PT=0）

    for line in sdp.lines() {
        let trimmed = line.trim();

        // c= 行：同时兼容 IP4 / IP6
        if let Some(rest) = trimmed
            .strip_prefix("c=IN IP4 ")
            .or_else(|| trimmed.strip_prefix("c=IN IP6 "))
        {
            device_ip = rest.trim().to_string();
        // m=audio 行：提取端口
        } else if let Some(rest) = trimmed.strip_prefix("m=audio ") {
            if let Some(p) = rest.split_whitespace().next() {
                device_port = p.parse().unwrap_or(0);
            }
        // y= 行：SSRC
        } else if let Some(rest) = trimmed.strip_prefix("y=") {
            ssrc = rest.trim().to_string();
        // a=rtpmap 行：识别编解码器（按 encoding name 匹配，大小写不敏感）
        } else if trimmed.starts_with("a=rtpmap:") {
            let upper = trimmed.to_uppercase();
            if upper.contains("PCMA") {
                codec = AudioCodec::PCMA;
            } else if upper.contains("SVAC") {
                codec = AudioCodec::SVAC;
            } else if upper.contains("AAC") {
                codec = AudioCodec::AAC;
            } else if upper.contains("G7221") || upper.contains("G722.1") {
                codec = AudioCodec::G7221;
            } else if upper.contains("G723") {
                codec = AudioCodec::G7231;
            } else if upper.contains("G729") {
                codec = AudioCodec::G729;
            }
            // PCMU/0 是默认值，无需显式处理
        }
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

// ── 抓拍 SDP ─────────────────────────────────────────────────────────────────

/// 抓拍会话信息（GB28181-2016 §9.14）
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    /// 抓拍图片的 URL（从 SDP `u=` 字段解析，含冒号后的版本号）
    pub image_url: String,
    /// 纯 URI 部分（`u=` 中最后一个冒号之前的内容）
    pub description: String,
}

/// 构建抓拍 INVITE 的 SDP offer
///
/// GB28181-2016 §9.14 / GB28181-2022 §9.14：平台向设备发起抓拍请求。
/// `s=SnapShot`，`u=` 携带上传 URL，`y=` 携带 SSRC。
///
/// # SDP 结构
/// ```text
/// v=0
/// o=- <session_id> <session_id> IN IP4 <local_ip>
/// s=SnapShot
/// u=http://<local_ip>:8080/snapshot?sn=<sn>:0
/// c=IN IP4 <local_ip>
/// t=0 0
/// m=image 0 RTP/AVP 98
/// a=rtpmap:98 jpeg/90000
/// y=<ssrc>
/// ```
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
/// - `description`：最后一个冒号前的纯 URI 部分
pub fn parse_snapshot_sdp(sdp: &str) -> SnapshotInfo {
    let raw = sdp
        .lines()
        .find(|l| l.starts_with("u="))
        .map(|l| l[2..].trim())
        .unwrap_or_default();

    let image_url = raw.to_string();
    let description = raw.rfind(':').map(|i| &raw[..i]).unwrap_or(raw).to_string();

    SnapshotInfo {
        image_url,
        description,
    }
}

/// 符合 GB/T 28181-2022 的抓拍配置信息（XML body 解析结果）
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
    // 第一步：GB18030 → UTF-8
    let (utf8_string, _, had_errors) = GB18030.decode(xml_bytes);
    if had_errors {
        eprintln!("警告：GB18030 解码出现替换字符");
    }

    let xml_str = utf8_string;

    // 第二步：流式 XML 解析
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
            Err(e) => return Err(format!("XML 解析错误: {}", e)),
            _ => {}
        }
    }

    if info.session_id.is_empty() {
        return Err("XML 中缺少会话 ID (SessionID)".to_string());
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
        let sdp = build_invite_sdp(
            "192.168.1.100",
            10000,
            "0000000001",
            SessionType::Play,
            None,
            None,
        )
        .unwrap();
        assert!(sdp.contains("s=Play"));
        assert!(sdp.contains("c=IN IP4 192.168.1.100"));
        assert!(sdp.contains("m=video 10000 RTP/AVP 96"));
        assert!(sdp.contains("a=recvonly"));
        assert!(sdp.contains("a=rtpmap:96 PS/90000"));
        assert!(sdp.contains("y=0000000001"));
        // 确保 t= 字段只出现一次（修复旧版重复 t=0 0 bug）
        assert_eq!(sdp.matches("t=0 0").count(), 1);
    }

    #[test]
    fn invite_sdp_playback() {
        let sdp = build_invite_sdp(
            "10.0.0.1",
            20000,
            "1234567890",
            SessionType::Playback,
            Some((1000000000, 1000003600)),
            None,
        )
        .unwrap();
        assert!(sdp.contains("s=Playback"));
        assert!(sdp.contains("t=1000000000 1000003600"));
        assert!(sdp.contains("m=video 20000"));
        assert!(sdp.contains("y=1234567890"));
    }

    #[test]
    fn invite_sdp_download_with_speed() {
        let sdp = build_invite_sdp(
            "10.0.0.1",
            20000,
            "1234567890",
            SessionType::Download,
            Some((1000000000, 1000003600)),
            Some(4),
        )
        .unwrap();
        assert!(sdp.contains("s=Download"));
        assert!(sdp.contains("a=downloadspeed:4"));
    }

    #[test]
    fn invite_sdp_playback_missing_time_range_returns_err() {
        let result = build_invite_sdp(
            "10.0.0.1",
            20000,
            "1234567890",
            SessionType::Playback,
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn invite_sdp_ipv6() {
        let sdp =
            build_invite_sdp("::1", 30000, "0000000001", SessionType::Play, None, None).unwrap();
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
            SessionType::Play,
            None,
            None,
        )
        .unwrap();
        assert!(sdp.contains("s=Play"));
        assert!(sdp.contains("a=sendonly"));
        assert!(sdp.contains("o=34020000001320000001"));
        assert!(sdp.contains("c=IN IP4 192.168.1.50"));
        assert!(sdp.contains("y=0000000001"));
        assert_eq!(sdp.matches("t=0 0").count(), 1);
    }

    #[test]
    fn sdp_answer_playback() {
        let sdp = build_sdp_answer(
            "192.168.1.50",
            10000,
            "0000000001",
            "34020000001320000001",
            SessionType::Playback,
            Some((1000000000, 1000003600)),
            None,
        )
        .unwrap();
        assert!(sdp.contains("s=Playback"));
        assert!(sdp.contains("t=1000000000 1000003600"));
    }

    #[test]
    fn sdp_answer_download_with_filesize() {
        let sdp = build_sdp_answer(
            "192.168.1.50",
            10000,
            "0000000001",
            "34020000001320000001",
            SessionType::Download,
            Some((1000000000, 1000003600)),
            Some(1048576),
        )
        .unwrap();
        assert!(sdp.contains("s=Download"));
        assert!(sdp.contains("a=filesize:1048576"));
    }

    #[test]
    fn sdp_answer_download_missing_filesize_returns_err() {
        let result = build_sdp_answer(
            "192.168.1.50",
            10000,
            "0000000001",
            "device1",
            SessionType::Download,
            Some((1000000000, 1000003600)),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn sdp_answer_ipv6() {
        let sdp = build_sdp_answer(
            "fe80::1",
            10000,
            "0000000001",
            "device1",
            SessionType::Play,
            None,
            None,
        )
        .unwrap();
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
        let (ip, port) = parse_sdp_destination(sdp, &GBMedia::Video).unwrap();
        assert_eq!(ip, "192.168.1.100");
        assert_eq!(port, 10000);
    }

    #[test]
    fn parse_destination_ipv6() {
        let sdp = "c=IN IP6 ::1\r\nm=video 20000 RTP/AVP 96\r\n";
        let (ip, port) = parse_sdp_destination(sdp, &GBMedia::Video).unwrap();
        assert_eq!(ip, "::1");
        assert_eq!(port, 20000);
    }

    #[test]
    fn parse_destination_missing_c_line_returns_err() {
        let sdp = "v=0\r\nm=video 5000 RTP/AVP 96\r\n";
        assert!(parse_sdp_destination(sdp, &GBMedia::Video).is_err());
    }

    // ── AudioCodec rtpmap ────────────────────────────────────────────────────

    #[test]
    fn audio_codec_rtpmap_static_pt() {
        // 静态 PT（RFC 3551）
        assert_eq!(AudioCodec::PCMU.rtpmap(), (0, "PCMU/8000"));
        assert_eq!(AudioCodec::PCMA.rtpmap(), (8, "PCMA/8000"));
        assert_eq!(AudioCodec::G7231.rtpmap(), (4, "G723/8000"));
        assert_eq!(AudioCodec::G729.rtpmap(), (18, "G729/8000"));
    }

    #[test]
    fn audio_codec_rtpmap_dynamic_pt() {
        // 动态 PT（GB28181 惯例）
        assert_eq!(AudioCodec::AAC.rtpmap(), (102, "AAC/8000"));
        assert_eq!(AudioCodec::G7221.rtpmap(), (104, "G7221/16000"));
        assert_eq!(AudioCodec::SVAC.rtpmap(), (100, "SVAC/8000"));
    }

    // ── build_audio_invite_sdp ───────────────────────────────────────────────

    #[test]
    fn audio_invite_sdp_has_video_and_audio() {
        let sdp =
            build_audio_invite_sdp("192.168.1.1", 10000, 20000, AudioCodec::PCMU, "0000000001");
        assert!(sdp.contains("m=video 10000"));
        assert!(sdp.contains("m=audio 20000"));
        assert!(sdp.contains("a=rtpmap:0 PCMU/8000"));
        assert!(sdp.contains("a=sendonly"));
        assert_eq!(sdp.matches("y=0000000001").count(), 2); // video + audio 各一个
    }

    #[test]
    fn audio_invite_sdp_pcma_codec() {
        let sdp = build_audio_invite_sdp("10.0.0.1", 10000, 20000, AudioCodec::PCMA, "0000000001");
        assert!(sdp.contains("a=rtpmap:8 PCMA/8000"));
    }

    #[test]
    fn audio_invite_sdp_aac_codec() {
        let sdp = build_audio_invite_sdp("10.0.0.1", 10000, 20000, AudioCodec::AAC, "0000000001");
        assert!(sdp.contains("a=rtpmap:102 AAC/8000"));
    }

    #[test]
    fn audio_invite_sdp_g722_1_codec() {
        let sdp = build_audio_invite_sdp("10.0.0.1", 10000, 20000, AudioCodec::G7221, "0000000001");
        assert!(sdp.contains("a=rtpmap:104 G7221/16000"));
    }

    #[test]
    fn audio_invite_sdp_svac_codec() {
        let sdp = build_audio_invite_sdp("10.0.0.1", 10000, 20000, AudioCodec::SVAC, "0000000001");
        assert!(sdp.contains("a=rtpmap:100 SVAC/8000"));
    }

    // ── build_broadcast_answer_sdp ───────────────────────────────────────────

    #[test]
    fn broadcast_answer_sdp_has_recvonly() {
        let sdp = build_broadcast_answer_sdp("192.168.1.1", 30000, AudioCodec::PCMU, "0000000001");
        assert!(sdp.contains("s=Broadcast"));
        assert!(sdp.contains("m=audio 30000"));
        assert!(sdp.contains("a=recvonly"));
        assert!(sdp.contains("a=rtpmap:0 PCMU/8000"));
    }

    // ── parse_audio_sdp ──────────────────────────────────────────────────────

    #[test]
    fn parse_audio_sdp_pcma() {
        let sdp = "c=IN IP4 192.168.1.50\r\nm=audio 30000 RTP/AVP 8\r\na=rtpmap:8 PCMA/8000\r\ny=0000000001\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.device_ip, "192.168.1.50");
        assert_eq!(info.device_port, 30000);
        assert_eq!(info.ssrc, "0000000001");
        assert_eq!(info.codec, AudioCodec::PCMA);
    }

    #[test]
    fn parse_audio_sdp_pcmu_default() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 0\r\ny=0000000002\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::PCMU);
    }

    #[test]
    fn parse_audio_sdp_aac() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 102\r\na=rtpmap:102 AAC/8000\r\ny=0000000003\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::AAC);
    }

    #[test]
    fn parse_audio_sdp_g7221() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 104\r\na=rtpmap:104 G7221/16000\r\ny=0000000004\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::G7221);
    }

    #[test]
    fn parse_audio_sdp_svac() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 100\r\na=rtpmap:100 SVAC/8000\r\ny=0000000005\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::SVAC);
    }

    #[test]
    fn parse_audio_sdp_g723() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 4\r\na=rtpmap:4 G723/8000\r\ny=0000000006\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::G7231);
    }

    #[test]
    fn parse_audio_sdp_g729() {
        let sdp = "c=IN IP4 10.0.0.1\r\nm=audio 5000 RTP/AVP 18\r\na=rtpmap:18 G729/8000\r\ny=0000000007\r\n";
        let info = parse_audio_sdp(sdp).unwrap();
        assert_eq!(info.codec, AudioCodec::G729);
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
}
