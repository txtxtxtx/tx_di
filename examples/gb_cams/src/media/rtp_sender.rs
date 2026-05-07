//! RTP 封包 + UDP 发送器
//!
//! 将 PS 数据封装为 RTP 包并通过 UDP 发送。
//! - RTP payload type: 96 (dynamic, GB28181 约定)
//! - 时钟频率: 90kHz (MPEG 标准)
//! - 最大包长: 1400 字节 payload (避免 IP 分片)

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::warn;

/// 最大 RTP payload 长度（避免 IP 分片）
const MAX_RTP_PAYLOAD: usize = 1400;

/// RTP payload type（GB28181 约定 PT=96 为 PS 流）
const RTP_PAYLOAD_TYPE: u8 = 96;

/// RTP 发送器
pub struct RtpSender {
    socket: UdpSocket,
    target: SocketAddr,
    ssrc: u32,
    sequence: u16,
}

impl RtpSender {
    /// 创建 RTP 发送器
    ///
    /// 自动绑定本地 UDP 端口，向 target_ip:target_port 发送。
    pub async fn new(target_ip: &str, target_port: u16, ssrc_str: &str) -> anyhow::Result<Self> {
        let target: SocketAddr = format!("{}:{}", target_ip, target_port)
            .parse()
            .map_err(|e| anyhow::anyhow!("无效的目标地址 {}:{}: {}", target_ip, target_port, e))?;

        // 绑定本地端口（0 = 系统自动分配）
        let bind_addr: SocketAddr = if target.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        }
        .parse()?;

        let socket = UdpSocket::bind(bind_addr).await?;
        socket.connect(target).await?;

        // 解析 SSRC（10位数字字符串 → u32）
        let ssrc = ssrc_str.parse::<u32>().unwrap_or(0x12345678);

        Ok(Self {
            socket,
            target,
            ssrc,
            sequence: 0,
        })
    }

    /// 发送 PS 数据（自动分片为多个 RTP 包）
    ///
    /// # 参数
    /// - `ps_data`: 完整的 PS 包数据
    /// - `timestamp`: RTP 时间戳（90kHz 时钟）
    /// - `marker`: 标记位（通常帧的最后一包置 1）
    pub async fn send_ps(&mut self, ps_data: &[u8], timestamp: u32, _frame_seq: u32) -> anyhow::Result<()> {
        if ps_data.is_empty() {
            return Ok(());
        }

        // 计算需要分片的包数
        let total_packets = (ps_data.len() + MAX_RTP_PAYLOAD - 1) / MAX_RTP_PAYLOAD;

        for (i, chunk) in ps_data.chunks(MAX_RTP_PAYLOAD).enumerate() {
            let is_last = i == total_packets - 1;

            // 构建 RTP 包头（12 字节）
            let header = build_rtp_header(
                self.sequence,
                timestamp,
                is_last,
                self.ssrc,
            );

            // 发送：RTP header + payload
            let mut packet = Vec::with_capacity(12 + chunk.len());
            packet.extend_from_slice(&header);
            packet.extend_from_slice(chunk);

            self.socket.send_to(&packet, self.target).await?;

            self.sequence = self.sequence.wrapping_add(1);
        }

        Ok(())
    }
}

/// 构建 RTP 包头（12 字节）
///
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |V=2|P|X|  CC   |M|     PT      |       sequence number         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                           timestamp                           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                             SSRC                              |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
fn build_rtp_header(sequence: u16, timestamp: u32, marker: bool, ssrc: u32) -> [u8; 12] {
    let mut header = [0u8; 12];

    // V=2, P=0, X=0, CC=0 → byte 0 = 0x80
    header[0] = 0x80;

    // M (marker) + PT
    header[1] = if marker {
        RTP_PAYLOAD_TYPE | 0x80
    } else {
        RTP_PAYLOAD_TYPE
    };

    // Sequence number (big-endian)
    header[2] = (sequence >> 8) as u8;
    header[3] = sequence as u8;

    // Timestamp (big-endian)
    header[4] = (timestamp >> 24) as u8;
    header[5] = (timestamp >> 16) as u8;
    header[6] = (timestamp >> 8) as u8;
    header[7] = timestamp as u8;

    // SSRC (big-endian)
    header[8] = (ssrc >> 24) as u8;
    header[9] = (ssrc >> 16) as u8;
    header[10] = (ssrc >> 8) as u8;
    header[11] = ssrc as u8;

    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtp_header() {
        let header = build_rtp_header(1, 90000, true, 0x12345678);
        assert_eq!(header[0], 0x80); // V=2
        assert_eq!(header[1], 0xE0 | RTP_PAYLOAD_TYPE); // M=1 + PT
        assert_eq!(&header[2..4], &[0, 1]); // seq=1
        assert_eq!(&header[8..12], &[0x12, 0x34, 0x56, 0x78]); // SSRC
    }
}
