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
