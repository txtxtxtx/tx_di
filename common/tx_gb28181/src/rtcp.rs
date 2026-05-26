//! RTCP 包解析与流媒体质量统计
//!
//! 实现 RTCP (RFC 3550) SR/RR 报文的解析，提供实时码率、丢包率、抖动、
//! 往返时延 (RTT) 等关键 QoS 指标的计算。
//!
//! ## 报文类型
//!
//! | PT  | 名称 | 说明 |
//! |-----|------|------|
//! | 200 | SR   | Sender Report — 发送端报告（含 NTP/RTP 时间戳、包/字节计数） |
//! | 201 | RR   | Receiver Report — 接收端报告（含丢包率、抖动等） |
//! | 202 | SDES | Source Description — 源描述（CNAME） |
//! | 203 | BYE  | Goodbye — 会话结束 |
//! | 204 | APP  | Application-defined |
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use tx_gb28181::rtcp::RtcpParser;
//!
//! let mut stats = RtcpStats::new();
//! let result = RtcpParser::parse(&rtcp_packet_bytes, &mut stats);
//! ```
//!
//! ## 字段含义
//!
//! | 字段 | 含义 | 来源 |
//! |------|------|------|
//! | `fraction_lost` | 丢包率（0-255，除以256得百分比） | RR report block |
//! | `cumulative_lost` | 累计丢包数 | RR report block |
//! | `jitter` | 到达间隔抖动（时间戳单位，需除以采样率） | RR report block |
//! | `rtt_ms` | 往返时延（毫秒） | 通过 SR NTP + DLSR 计算 |
//! | `bitrate_kbps` | 瞬时码率（kbps） | SR packet_count + octet_count |
//! | `packets` | 累计发送/接收包数 | SR |
//! | `octets` | 累计发送/接收字节数 | SR |

use std::collections::HashMap;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// RTCP 统计数据结构
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// RTCP 实时统计快照
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RtcpStats {
    /// 丢包率（0.0 ~ 1.0）
    pub fraction_lost: f64,
    /// 累计丢包数
    pub cumulative_lost: u32,
    /// 到达间隔抖动（采样时间戳单位）
    pub jitter: u32,
    /// 往返时延（毫秒，仅当收到 RR 且包含 LSR+DLSR 时有效）
    pub rtt_ms: Option<f64>,
    /// 瞬时码率（kbps）
    pub bitrate_kbps: f64,
    /// 累计包数
    pub packets: u32,
    /// 累计字节数（含 RTP 头+载荷）
    pub octets: u32,
    /// 最后更新时间（Unix 秒）
    pub last_updated: f64,
}

/// 每 SSRC 的历史数据（用于计算瞬时码率）
#[derive(Debug, Default)]
struct SsrcHistory {
    last_packets: u32,
    last_octets: u32,
    last_time: f64,
}

/// RTCP 统计收集器
#[derive(Debug, Default)]
pub struct RtcpStatsCollector {
    /// 当前统计快照（合并所有 SSRC）
    pub stats: RtcpStats,
    /// 用于码率计算的 SSRC 历史数据
    history: HashMap<u32, SsrcHistory>,
}

impl RtcpStatsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取统计快照的引用
    pub fn snapshot(&self) -> &RtcpStats {
        &self.stats
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// RTCP 包解析器
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// RTCP 包类型常量
const RTCP_SR: u8 = 200;
const RTCP_RR: u8 = 201;
const RTCP_SDES: u8 = 202;
const RTCP_BYE: u8 = 203;

/// RTCP 公共头大小（字节）：V/P/RC(1) + PT(1) + length(2) = 4
const RTCP_HEADER_SIZE: usize = 4;
/// SSRC 字段大小：4 字节
const SSRC_SIZE: usize = 4;
/// SR 发送者信息大小：NTP(8) + RTP TS(4) + packets(4) + octets(4) = 20
const SR_SENDER_INFO_SIZE: usize = 20;
/// RR report block 大小：SSRC(4) + frac_lost(1) + cum_lost(3) + ext_seq(4) + jitter(4) + LSR(4) + DLSR(4) = 24
const REPORT_BLOCK_SIZE: usize = 24;

/// 解析结果
#[derive(Debug, Clone)]
pub enum RtcpReport {
    /// Sender Report
    Sr {
        ssrc: u32,
        /// NTP 时间戳（高32位）
        ntp_msw: u32,
        /// NTP 时间戳（低32位）
        ntp_lsw: u32,
        /// RTP 时间戳
        rtp_ts: u32,
        /// 累计包数
        packets: u32,
        /// 累计字节数
        octets: u32,
        /// 报告块列表
        reports: Vec<ReceptionReport>,
    },
    /// Receiver Report
    Rr {
        ssrc: u32,
        reports: Vec<ReceptionReport>,
    },
}

/// 接收报告块
#[derive(Debug, Clone, Default)]
pub struct ReceptionReport {
    pub ssrc: u32,
    /// 丢包率（0-255，/256 = 实际比例）
    pub fraction_lost: u8,
    /// 累计丢包数（24位有符号整数）
    pub cumulative_lost: i32,
    /// 扩展最高序列号
    pub ext_highest_seq: u32,
    /// 到达间隔抖动
    pub jitter: u32,
    /// Last SR timestamp（NTP 中32位）
    pub lsr: u32,
    /// Delay since Last SR（1/65536 秒单位）
    pub dlsr: u32,
}

impl RtcpReport {
    /// 解析 RTCP 复合包（可能包含多个子包）
    pub fn parse(data: &[u8]) -> Result<Vec<RtcpReport>, String> {
        let mut reports = Vec::new();
        let mut offset = 0;

        while offset + RTCP_HEADER_SIZE <= data.len() {
            // 解析公共头
            let header_byte = data[offset];
            let version = (header_byte >> 6) & 0x03;
            if version != 2 {
                return Err(format!("不支持的 RTCP 版本: {version}"));
            }

            let pt = data[offset + 1];
            let length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            let packet_len = (length + 1) * 4; // length 是 32-bit words - 1，含公共头

            if offset + packet_len > data.len() {
                break; // 包不完整
            }

            let packet_data = &data[offset..offset + packet_len];

            match pt {
                RTCP_SR => {
                    if let Some(report) = parse_sr(packet_data) {
                        reports.push(report);
                    }
                }
                RTCP_RR => {
                    if let Some(report) = parse_rr(packet_data) {
                        reports.push(report);
                    }
                }
                // SDES / BYE / APP 暂不解析
                _ => {}
            }

            offset += packet_len;
        }

        if reports.is_empty() {
            return Err("未找到有效的 RTCP 包".to_string());
        }
        Ok(reports)
    }

    /// 将解析结果应用到统计收集器
    pub fn apply_to_collector(reports: &[RtcpReport], collector: &mut RtcpStatsCollector) {
        let now = current_time_secs();

        for report in reports {
            match report {
                RtcpReport::Sr { ssrc, packets, octets, ntp_msw, ntp_lsw, reports: rbs, .. } => {
                    // 更新发送端统计
                    apply_sr_to_stats(*ssrc, *packets, *octets, now, collector);

                    // 处理报告块（用于 RTT 计算）
                    for rb in rbs {
                        // RTT = NTP_now - LSR - DLSR
                        // LSR 是 NTP 中32位，DLSR 是 1/65536 秒
                        if rb.lsr > 0 && rb.dlsr > 0 {
                            let ntp_now_mid = ((*ntp_msw as u64) << 32 | *ntp_lsw as u64 >> 32) as u32;
                            let rtt = ntp_now_mid.wrapping_sub(rb.lsr) as f64 - (rb.dlsr as f64 / 65536.0);
                            if rtt < 60.0 && rtt > 0.0 {
                                // 合理范围：0 ~ 60 秒
                                collector.stats.rtt_ms = Some((rtt * 1000.0).max(0.0));
                            }
                        }

                        // 更新接收报告
                        collector.stats.fraction_lost = rb.fraction_lost as f64 / 256.0;
                        collector.stats.cumulative_lost = rb.cumulative_lost.max(0) as u32;
                        collector.stats.jitter = rb.jitter;
                    }
                }
                RtcpReport::Rr { reports: rbs, .. } => {
                    for rb in rbs {
                        collector.stats.fraction_lost = rb.fraction_lost as f64 / 256.0;
                        collector.stats.cumulative_lost = rb.cumulative_lost.max(0) as u32;
                        collector.stats.jitter = rb.jitter;
                    }
                }
            }
        }

        collector.stats.last_updated = now;
    }
}

/// 应用 SR 发送端数据到统计（计算码率）
fn apply_sr_to_stats(
    ssrc: u32,
    packets: u32,
    octets: u32,
    now: f64,
    collector: &mut RtcpStatsCollector,
) {
    let history = collector.history.entry(ssrc).or_default();

    if history.last_time > 0.0 && packets >= history.last_packets {
        let dt = now - history.last_time;
        let d_octets = octets.saturating_sub(history.last_octets);

        if dt > 0.0 {
            // 码率 = 字节变化量 * 8 / 时间差 / 1000 (kbps)
            collector.stats.bitrate_kbps = (d_octets as f64 * 8.0) / dt / 1000.0;
        }
    }

    history.last_packets = packets;
    history.last_octets = octets;
    history.last_time = now;

    collector.stats.packets = packets;
    collector.stats.octets = octets;
}

/// 获取当前 Unix 时间（秒）
fn current_time_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 解析器内部函数
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 解析 SR 包
fn parse_sr(data: &[u8]) -> Option<RtcpReport> {
    if data.len() < RTCP_HEADER_SIZE + SSRC_SIZE + SR_SENDER_INFO_SIZE {
        return None;
    }

    let rc = data[0] & 0x1F; // Report Count
    let ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    let ntp_msw = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let ntp_lsw = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    let rtp_ts = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let packets = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    let octets = u32::from_be_bytes([data[24], data[25], data[26], data[27]]);

    let mut reports = Vec::new();
    let mut offset = RTCP_HEADER_SIZE + SSRC_SIZE + SR_SENDER_INFO_SIZE;

    for _ in 0..rc {
        if offset + REPORT_BLOCK_SIZE <= data.len() {
            reports.push(parse_report_block(&data[offset..]));
            offset += REPORT_BLOCK_SIZE;
        }
    }

    Some(RtcpReport::Sr {
        ssrc,
        ntp_msw,
        ntp_lsw,
        rtp_ts,
        packets,
        octets,
        reports,
    })
}

/// 解析 RR 包
fn parse_rr(data: &[u8]) -> Option<RtcpReport> {
    if data.len() < RTCP_HEADER_SIZE + SSRC_SIZE {
        return None;
    }

    let rc = data[0] & 0x1F;
    let ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    let mut reports = Vec::new();
    let mut offset = RTCP_HEADER_SIZE + SSRC_SIZE;

    for _ in 0..rc {
        if offset + REPORT_BLOCK_SIZE <= data.len() {
            reports.push(parse_report_block(&data[offset..]));
            offset += REPORT_BLOCK_SIZE;
        }
    }

    Some(RtcpReport::Rr { ssrc, reports })
}

/// 解析 report block
fn parse_report_block(data: &[u8]) -> ReceptionReport {
    let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let fraction_lost = data[4];
    let cumulative_lost: i32 = {
        let raw = u32::from_be_bytes([0, data[5], data[6], data[7]]);
        // 24-bit signed → 扩展到 32-bit signed
        if raw & 0x0080_0000 != 0 {
            -(((!raw) & 0x00FF_FFFF) as i32 + 1)
        } else {
            raw as i32
        }
    };
    let ext_highest_seq = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let jitter = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    let lsr = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let dlsr = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

    ReceptionReport {
        ssrc,
        fraction_lost,
        cumulative_lost,
        ext_highest_seq,
        jitter,
        lsr,
        dlsr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert!(RtcpReport::parse(&[]).is_err());
    }

    #[test]
    fn test_parse_rr_minimal() {
        // 构造一个最小的 RR 包（version=2, no padding, RC=1, PT=201, length=7）
        // length=7 → packet_len = (7+1)*4 = 32 bytes
        let mut packet = vec![0u8; RTCP_HEADER_SIZE + SSRC_SIZE + REPORT_BLOCK_SIZE];
        packet[0] = 0x80 | 1; // V=2, P=0, RC=1
        packet[1] = RTCP_RR;
        packet[2] = 0x00;
        packet[3] = 7; // length = (4+4+24)/4 - 1 = 32/4 - 1 = 7
        // SSRC (bytes 4-7)
        packet[4] = 0x12;
        packet[5] = 0x34;
        packet[6] = 0x56;
        packet[7] = 0x78;
        // Report block (bytes 8-31) — all zeros is valid

        let reports = RtcpReport::parse(&packet).unwrap();
        assert_eq!(reports.len(), 1);
        match &reports[0] {
            RtcpReport::Rr { ssrc, reports: rbs } => {
                assert_eq!(*ssrc, 0x12345678);
                assert_eq!(rbs.len(), 1);
                assert_eq!(rbs[0].fraction_lost, 0);
            }
            _ => panic!("Expected RR"),
        }
    }

    #[test]
    fn test_parse_sr() {
        // 构造 SR 包：V=2, RC=0, PT=200
        let total_size = RTCP_HEADER_SIZE + SSRC_SIZE + SR_SENDER_INFO_SIZE;
        let length = total_size / 4 - 1; // 28/4 - 1 = 6
        let mut packet = vec![0u8; total_size];
        packet[0] = 0x80; // V=2, RC=0
        packet[1] = RTCP_SR;
        packet[2] = 0x00;
        packet[3] = length as u8;
        // SR sender info
        packet[12] = 0x00;
        packet[13] = 0x00;
        packet[14] = 0x00;
        packet[15] = 0x64; // RTP TS = 100
        packet[20] = 0x00;
        packet[21] = 0x00;
        packet[22] = 0x03;
        packet[23] = 0xE8; // packets = 1000
        packet[24] = 0x00;
        packet[25] = 0x01;
        packet[26] = 0x86;
        packet[27] = 0xA0; // octets = 100000

        let reports = RtcpReport::parse(&packet).unwrap();
        match &reports[0] {
            RtcpReport::Sr { packets, octets, .. } => {
                assert_eq!(*packets, 1000);
                assert_eq!(*octets, 100000);
            }
            _ => panic!("Expected SR"),
        }
    }

    #[test]
    fn test_stats_collector_bitrate() {
        let mut collector = RtcpStatsCollector::new();

        // 模拟两次 SR，间隔 1 秒
        let sr1 = RtcpReport::Sr {
            ssrc: 1,
            ntp_msw: 0,
            ntp_lsw: 0,
            rtp_ts: 0,
            packets: 100,
            octets: 100000,
            reports: vec![],
        };
        let sr2 = RtcpReport::Sr {
            ssrc: 1,
            ntp_msw: 0,
            ntp_lsw: 0,
            rtp_ts: 0,
            packets: 200,
            octets: 200000,
            reports: vec![],
        };

        // 第一次解析
        let reports1 = vec![sr1];
        RtcpReport::apply_to_collector(&reports1, &mut collector);
        assert_eq!(collector.stats.packets, 100);

        // 第二次解析
        let reports2 = vec![sr2];
        RtcpReport::apply_to_collector(&reports2, &mut collector);
        assert_eq!(collector.stats.packets, 200);
        // 码率应该 > 0 (100KB in ~0 sec → high bitrate)
        assert!(collector.stats.bitrate_kbps > 0.0);
    }

    #[test]
    fn test_report_block_loss() {
        let mut data = [0u8; REPORT_BLOCK_SIZE];
        data[4] = 128; // fraction_lost = 128 = 50%
        data[5] = 0x00;
        data[6] = 0x00;
        data[7] = 0x0A; // cumulative_lost = 10

        let rb = parse_report_block(&data);
        assert_eq!(rb.fraction_lost, 128);
        assert_eq!(rb.cumulative_lost, 10);
    }
}
