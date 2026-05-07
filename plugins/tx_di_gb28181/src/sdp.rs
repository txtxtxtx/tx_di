//! SDP 工具函数（GB28181 规范）
//!
//! GB28181 使用标准 SDP，额外规定：
//! - RTP payload type 96 = PS（MPEG-PS 打包）
//! - `y=` 字段携带 SSRC（10 位数字字符串）
//! - `f=` 字段描述视频参数（可选）

/// 判断 IP 字符串是 IPv6 还是 IPv4，返回 `("IP6", addr)` 或 `("IP4", addr)`
///
/// IPv6 地址在 SDP `c=` / `o=` 字段中不需要方括号，直接裸写即可（RFC 4566 §5.7）。
pub(crate) fn ip_net_type(ip: &str) -> (&'static str, &str) {
    if ip.contains(':') {
        ("IP6", ip)
    } else {
        ("IP4", ip)
    }
}

/// 构建点播 INVITE 的 SDP offer
///
/// 平台向设备发送 INVITE 时携带此 SDP，告知设备将流推到哪里。
///
/// # 参数
/// - `local_ip`：媒体服务器 IP（接收 RTP 流的地址，IPv4 或 IPv6 均可）
/// - `rtp_port`：接收 RTP 的端口
/// - `ssrc`：SSRC 标识符（10 位数字，用于区分多路流）
/// - `is_realtime`：`true` = 实时点播（`s=Play`），`false` = 历史回放（`s=Playback`）
pub fn build_invite_sdp(local_ip: &str, rtp_port: u16, ssrc: &str, is_realtime: bool) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name = if is_realtime { "Play" } else { "Playback" };
    let (addrtype, addr) = ip_net_type(local_ip);

    format!(
        "v=0\r\n\
o=- {session_id} {session_id} IN {addrtype} {addr}\r\n\
s={session_name}\r\n\
c=IN {addrtype} {addr}\r\n\
t=0 0\r\n\
m=video {rtp_port} RTP/AVP 96\r\n\
a=recvonly\r\n\
a=rtpmap:96 PS/90000\r\n\
y={ssrc}\r\n",
        session_id = session_id,
        addrtype = addrtype,
        addr = addr,
        session_name = session_name,
        rtp_port = rtp_port,
        ssrc = ssrc
    )
}

/// 构建设备回复给平台的 SDP answer（200 OK 中携带）
///
/// 设备接收到 INVITE 后，用此 SDP 告知平台"我将从哪里推流"。
///
/// # 参数
/// - `local_ip`：设备本地 IP（IPv4 或 IPv6 均可）
/// - `rtp_port`：设备推流 RTP 端口
/// - `ssrc`：与 offer 中相同的 SSRC
/// - `device_id`：设备编号，填入 `o=` 用户名字段
/// - `is_realtime`：`true` = 实时（`s=Play`），`false` = 回放（`s=Playback`）
pub fn build_sdp_answer(
    local_ip: &str,
    rtp_port: u16,
    ssrc: &str,
    device_id: &str,
    is_realtime: bool,
) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name = if is_realtime { "Play" } else { "Playback" };
    let (addrtype, addr) = ip_net_type(local_ip);

    format!(
        "v=0\r\n\
o={device_id} {session_id} {session_id} IN {addrtype} {addr}\r\n\
s={session_name}\r\n\
c=IN {addrtype} {addr}\r\n\
t=0 0\r\n\
m=video {rtp_port} RTP/AVP 96\r\n\
a=sendonly\r\n\
a=rtpmap:96 PS/90000\r\n\
y={ssrc}\r\n",
        device_id = device_id,
        session_id = session_id,
        addrtype = addrtype,
        addr = addr,
        session_name = session_name,
        rtp_port = rtp_port,
        ssrc = ssrc
    )
}

/// 从 SDP 中解析 SSRC（`y=` 字段）
pub fn parse_sdp_ssrc(sdp: &str) -> Option<String> {
    sdp.lines()
        .find(|l| l.starts_with("y="))
        .map(|l| l[2..].trim().to_string())
}

/// 从 SDP 中解析媒体目标 IP 和端口
///
/// 返回 `(ip, port)` — 平台希望设备推流到哪里。
/// 同时支持 IPv4（`c=IN IP4 ...`）和 IPv6（`c=IN IP6 ...`）。
pub fn parse_sdp_destination(sdp: &str) -> (String, u16) {
    let mut ip = String::new();
    let mut port = 0u16;
    for line in sdp.lines() {
        if let Some(rest) = line
            .strip_prefix("c=IN IP4 ")
            .or_else(|| line.strip_prefix("c=IN IP6 "))
        {
            ip = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("m=video ") {
            if let Some(p) = rest.split_whitespace().next() {
                port = p.parse().unwrap_or(0);
            }
        }
    }
    if ip.is_empty() {
        ip = "0.0.0.0".to_string();
    }
    (ip, port)
}

// ── 抓拍 SDP ─────────────────────────────────────────────────────────────────

/// 抓拍会话信息
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    /// 抓拍图片的 URL（从 SDP `u=` 字段解析，含冒号后的版本号）
    pub image_url: String,
    /// 抓拍描述（`u=` 中冒号前的 URL 部分）
    pub description: String,
}

// ── 语音广播/对讲 SDP ─────────────────────────────────────────────────────────

/// 音频编码类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    /// G.711 PCMU (mulaw)
    Pcmu,
    /// G.711 PCMA (alaw)
    Pcma,
    /// G.726
    G726,
}

impl AudioCodec {
    /// 返回 RTPMAP payload type 和编码名
    pub fn rtpmap(&self) -> (u8, &'static str) {
        match self {
            AudioCodec::Pcmu => (0, "PCMU/8000"),
            AudioCodec::Pcma => (8, "PCMA/8000"),
            AudioCodec::G726 => (96, "G726-16/8000"),
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
    let description = raw
        .rfind(':')
        .map(|i| &raw[..i])
        .unwrap_or(raw)
        .to_string();

    SnapshotInfo {
        image_url,
        description,
    }
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
        let sdp = build_sdp_answer("192.168.1.50", 10000, "0000000001", "34020000001320000001", true);
        assert!(sdp.contains("s=Play"));
        assert!(sdp.contains("a=sendonly"));
        assert!(sdp.contains("o=34020000001320000001"));
        assert!(sdp.contains("c=IN IP4 192.168.1.50"));
    }

    #[test]
    fn sdp_answer_playback() {
        let sdp = build_sdp_answer("192.168.1.50", 10000, "0000000001", "34020000001320000001", false);
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
        let sdp = build_audio_invite_sdp("192.168.1.1", 10000, 20000, AudioCodec::Pcmu, "0000000001");
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

