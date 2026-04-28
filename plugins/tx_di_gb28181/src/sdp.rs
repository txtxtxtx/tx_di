//! SDP 工具函数（GB28181 规范）
//!
//! GB28181 使用标准 SDP，额外规定：
//! - RTP payload type 96 = PS（MPEG-PS 打包）
//! - `y=` 字段携带 SSRC（10 位数字字符串）
//! - `f=` 字段描述视频参数（可选）

/// 构建点播 INVITE 的 SDP offer
///
/// 平台向设备发送 INVITE 时携带此 SDP，告知设备将流推到哪里。
///
/// # 参数
/// - `local_ip`：媒体服务器 IP（接收 RTP 流的地址）
/// - `rtp_port`：接收 RTP 的端口
/// - `ssrc`：SSRC 标识符（10 位数字，用于区分多路流）
/// - `is_realtime`：`true` = 实时点播（`s=Play`），`false` = 历史回放（`s=Playback`）
pub fn build_invite_sdp(local_ip: &str, rtp_port: u16, ssrc: &str, is_realtime: bool) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_name = if is_realtime { "Play" } else { "Playback" };

    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN IP4 {local_ip}\r\n\
         s={session_name}\r\n\
         c=IN IP4 {local_ip}\r\n\
         t=0 0\r\n\
         m=video {rtp_port} RTP/AVP 96\r\n\
         a=recvonly\r\n\
         a=rtpmap:96 PS/90000\r\n\
         y={ssrc}\r\n",
        session_id = session_id,
        local_ip = local_ip,
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
/// - `local_ip`：设备本地 IP
/// - `rtp_port`：设备推流 RTP 端口
/// - `ssrc`：与 offer 中相同的 SSRC
pub fn build_sdp_answer(local_ip: &str, rtp_port: u16, ssrc: &str) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN IP4 {local_ip}\r\n\
         s=Play\r\n\
         c=IN IP4 {local_ip}\r\n\
         t=0 0\r\n\
         m=video {rtp_port} RTP/AVP 96\r\n\
         a=sendonly\r\n\
         a=rtpmap:96 PS/90000\r\n\
         y={ssrc}\r\n",
        session_id = session_id,
        local_ip = local_ip,
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
/// 返回 `(ip, port)` — 平台希望设备推流到哪里
pub fn parse_sdp_destination(sdp: &str) -> (String, u16) {
    let mut ip = "0.0.0.0".to_string();
    let mut port = 0u16;
    for line in sdp.lines() {
        if let Some(rest) = line.strip_prefix("c=IN IP4 ") {
            ip = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("m=video ") {
            if let Some(p) = rest.split_whitespace().next() {
                port = p.parse().unwrap_or(0);
            }
        }
    }
    (ip, port)
}

// ── 抓拍 SDP ─────────────────────────────────────────────────────────────────

/// 抓拍会话信息
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    /// 抓拍图片的 URL（从 SDP 解析）
    pub image_url: String,
    /// 抓拍描述信息
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
/// a=sendonly 表示平台向设备发送音频
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

    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN IP4 {local_ip}\r\n\
         s=Play\r\n\
         c=IN IP4 {local_ip}\r\n\
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
        local_ip = local_ip,
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
pub fn build_broadcast_answer_sdp(local_ip: &str, audio_port: u16, codec: AudioCodec, ssrc: &str) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (audio_pt, audio_rtpmap) = codec.rtpmap();

    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN IP4 {local_ip}\r\n\
         s=Broadcast\r\n\
         c=IN IP4 {local_ip}\r\n\
         t=0 0\r\n\
         m=audio {audio_port} RTP/AVP {audio_pt}\r\n\
         a=recvonly\r\n\
         a=rtpmap:{audio_pt} {audio_rtpmap}\r\n\
         y={ssrc}\r\n",
        session_id = session_id,
        local_ip = local_ip,
        audio_port = audio_port,
        audio_pt = audio_pt,
        audio_rtpmap = audio_rtpmap,
        ssrc = ssrc
    )
}

/// 从对讲 SDP 中解析音频信息
pub fn parse_audio_sdp(sdp: &str) -> Option<AudioSessionInfo> {
    let mut device_ip = String::new();
    let mut device_port = 0u16;
    let mut ssrc = String::new();
    let mut codec = AudioCodec::Pcmu;

    for line in sdp.lines() {
        if let Some(rest) = line.strip_prefix("c=IN IP4 ") {
            device_ip = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("m=audio ") {
            if let Some(p) = rest.split_whitespace().next() {
                device_port = p.parse().unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("y=") {
            ssrc = rest.trim().to_string();
        } else if line.contains("PCMU") {
            codec = AudioCodec::Pcmu;
        } else if line.contains("PCMA") {
            codec = AudioCodec::Pcma;
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

/// 构建抓拍 INVITE 的 SDP offer
///
/// GB28181-2022 §9.14：平台向设备发起抓拍请求
/// s=SnapShot，u=描述，y=SSRC
pub fn build_snapshot_sdp(local_ip: &str, sn: u32) -> String {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let ssrc = format!("{:010}", sn);

    format!(
        "v=0\r\n\
         o=- {session_id} {session_id} IN IP4 {local_ip}\r\n\
         s=SnapShot\r\n\
         u=http://localhost:8080/snapshot?sn={sn}:0\r\n\
         c=IN IP4 {local_ip}\r\n\
         t=0 0\r\n\
         m=image 0 RTP/AVP 98\r\n\
         a=rtpmap:98 jpeg/90000\r\n\
         y={ssrc}\r\n",
        session_id = session_id,
        local_ip = local_ip,
        sn = sn,
        ssrc = ssrc
    )
}

/// 从 SDP 中解析抓拍图片 URL
///
/// 优先从 `u=` 字段解析，fallback 到 `a=filepath:` 属性
pub fn parse_snapshot_sdp(sdp: &str) -> SnapshotInfo {
    let image_url = sdp
        .lines()
        .find(|l| l.starts_with("u="))
        .map(|l| l[2..].trim().to_string())
        .unwrap_or_default();

    let description = sdp
        .lines()
        .find(|l| l.starts_with("u="))
        .map(|l| {
            l[2..]
                .trim()
                .split(' ')
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();

    SnapshotInfo {
        image_url,
        description,
    }
}
