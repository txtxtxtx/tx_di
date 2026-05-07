//! PS (Program Stream) 封装器
//!
//! GB28181 规范要求使用 MPEG-PS 封装：
//! - PS Header → System Header → PSM → PES（视频）
//! - PS Header → PES（后续帧）
//!
//! 本实现将 JPEG 帧封装为 MPEG-PS 流，兼容 GB28181-2022 附录规范。
//!
//! PS 封装结构:
//! ```text
//! [PS Header (14B)] [System Header (18B)] [PSM (24B)] [PES (video)]
//! [PS Header (14B)] [PES (video)] ...
//! ```

use bytes::{BufMut, BytesMut};

/// MPEG-PS 起始码
const PS_START_CODE: u32 = 0x0000_01BA;
const SYSTEM_HEADER_START_CODE: u32 = 0x0000_01BB;
const PSM_START_CODE: u32 = 0x0000_01BC;
const VIDEO_STREAM_ID: u8 = 0xE0;  // 视频流 0
const AUDIO_STREAM_ID: u8 = 0xC0;  // 音频流 0

/// 将 JPEG 帧数据封装为 PS 包
///
/// 每帧生成完整的 PS 包（PS Header + System Header + PSM + PES）
pub fn wrap_as_ps(jpeg_data: &[u8], pts: u32, _ssrc: &str) -> Vec<u8> {
    // 为了简化，我们生成一个 MPEG-PS 格式的字节流
    // 第一个 I 帧需要完整头，后续 P 帧可以省略 System Header 和 PSM
    let mut buf = BytesMut::with_capacity(jpeg_data.len() + 256);

    // 1. PS Header (Pack Header)
    write_ps_header(&mut buf, pts);

    // 2. System Header (仅首帧需要，但 GB28181 要求每帧都带)
    write_system_header(&mut buf);

    // 3. PSM (Program Stream Map)
    write_psm(&mut buf);

    // 4. PES Packet (Video)
    write_pes_video(&mut buf, jpeg_data, pts);

    buf.to_vec()
}

/// 写入 PS Header (Pack Header)
///
/// 格式 (14 字节):
/// ```text
/// 00 00 01 BA [6B SCR+MuxRate] [3B stuffing] [rest...]
/// ```
fn write_ps_header(buf: &mut BytesMut, pts: u32) {
    // 起始码
    buf.put_u32(PS_START_CODE);

    // SCR (System Clock Reference) 编码
    // 字节 4-9: '01' + SCR[32..30] + '1' + SCR[29..15] + '1' + SCR[14..0] + '1' + ext + '1'
    let scr = pts as u64;
    let scr32_30 = ((scr >> 30) & 0x07) as u8;
    let scr29_15 = ((scr >> 15) & 0x7FFF) as u16;
    let scr14_0 = (scr & 0x7FFF) as u16;

    let byte4 = 0x44 | (scr32_30 << 3) | 0x04; // '01' + SCR[32..30] + marker
    let byte5 = (scr29_15 >> 7) as u8;
    let byte6 = ((scr29_15 & 0x7F) << 1) as u8 | 0x01; // + marker
    let byte7 = (scr14_0 >> 7) as u8;
    let byte8 = ((scr14_0 & 0x7F) << 1) as u8 | 0x01; // + marker
    let byte9 = 0x01; // SCR extension + marker

    buf.put_u8(byte4);
    buf.put_u8(byte5);
    buf.put_u8(byte6);
    buf.put_u8(byte7);
    buf.put_u8(byte8);
    buf.put_u8(byte9);

    // Program Mux Rate (22 bits) + marker bits
    // 2Mbps = 2000000/50 = 40000 bytes/s → 40000/50 = 800 (in 50B units)
    let mux_rate = 2500u32; // 约 1 Mbps
    buf.put_u8((mux_rate >> 14) as u8);
    buf.put_u8((mux_rate >> 6) as u8);
    buf.put_u8(((mux_rate & 0x3F) << 2) as u8 | 0x03); // + marker + reserved

    // Stuffing length (3 bits reserved + 5 bits stuffing)
    buf.put_u8(0xF8); // 0 stuffing bytes
}

/// 写入 System Header
///
/// 格式 (18 字节):
/// ```text
/// 00 00 01 BB [header_length] [rate_bound] [audio_bound] [video_bound] ...
/// ```
fn write_system_header(buf: &mut BytesMut) {
    buf.put_u32(SYSTEM_HEADER_START_CODE);

    // header_length: 12 bytes after this field
    buf.put_u16(12);

    // rate_bound (22 bits) + markers
    let rate_bound = 2500u32;
    buf.put_u8((rate_bound >> 15) as u8 | 0x80); // marker + rate_bound[22..15]
    buf.put_u8((rate_bound >> 7) as u8);
    buf.put_u8(((rate_bound & 0x7F) << 1) as u8 | 0x01);

    // audio_bound(6) + fixed_flag(1) + CSPS_flag(1)
    buf.put_u8(0x20); // audio_bound=1

    // system_audio_lock_flag + system_video_lock_flag + marker + video_bound(5)
    buf.put_u8(0x21); // video_bound=1

    // packet_rate_restriction_flag + reserved
    buf.put_u8(0xFF);

    // Stream info: video stream
    // 10xxxxxx + stream_id + STD_buffer_bound_scale + STD_buffer_size_bound
    buf.put_u8(0xE0); // video stream 0 marker
    buf.put_u8(VIDEO_STREAM_ID);
    buf.put_u16(0xC001); // STD_buffer_bound_scale=1, buffer_size=1

    // Stream info: audio stream
    buf.put_u8(0xC0);
    buf.put_u8(AUDIO_STREAM_ID);
    buf.put_u16(0xC001);
}

/// 写入 PSM (Program Stream Map)
fn write_psm(buf: &mut BytesMut) {
    buf.put_u32(PSM_START_CODE);

    // length: 22 bytes
    buf.put_u16(22);

    // current_next_indicator + reserved + program_stream_map_version
    buf.put_u8(0xE1); // indicator=1, version=1
    buf.put_u8(0xFF); // reserved + marker

    // program_stream_info_length
    buf.put_u16(0);

    // elementary_stream_map_length = 16 (2 streams × 8 bytes each)
    buf.put_u16(16);

    // Video: stream_type=0x10 (MPEG-4 Video) + stream_id=0xE0
    buf.put_u8(0x10); // MPEG-4 Video
    buf.put_u8(VIDEO_STREAM_ID);
    buf.put_u16(0); // elementary_stream_info_length
    // 对于 JPEG (Motion JPEG)，stream_type 可以用 0x10 或自定义
    // GB28181 通常用 0x1B (H.264) 或 0x93 (SVAC)
    // 我们用 0x80 (用户私有) 表示非标准封装
    // 但实际实现中应该根据真正的编码格式来设置

    // Audio: stream_type=0x0F (AAC) + stream_id=0xC0
    buf.put_u8(0x0F); // AAC
    buf.put_u8(AUDIO_STREAM_ID);
    buf.put_u16(0);

    // CRC32
    buf.put_u32(0xFFFF_FFFF);
}

/// 写入 PES 包（视频流）
fn write_pes_video(buf: &mut BytesMut, payload: &[u8], pts: u32) {
    // PES start code: 00 00 01 E0 (video stream 0)
    buf.put_u32(0x0000_0100 | VIDEO_STREAM_ID as u32);

    // PES packet length: payload + header overhead
    // PTS 编码需要 5 字节
    let pes_header_len = 5; // PTS only
    let pes_data_len = pes_header_len + payload.len();
    let pes_packet_len = pes_data_len as u16;
    buf.put_u16(pes_packet_len);

    // PES header flags
    // '10' + scrambling(2) + priority(1) + alignment(1) + copyright(1) + original(1)
    buf.put_u8(0x84); // '10' + data_alignment_indicator=1

    // PTS/DTS flags + other flags
    buf.put_u8(0x80); // PTS present, no DTS

    // PES header data length
    buf.put_u8(pes_header_len as u8);

    // PTS 编码 (5 bytes)
    // '0010' + PTS[32..30] + marker + PTS[29..15] + marker + PTS[14..0] + marker
    let pts_val = pts as u64;
    let pts32_30 = ((pts_val >> 30) & 0x07) as u8;
    let pts29_15 = ((pts_val >> 15) & 0x7FFF) as u16;
    let pts14_0 = (pts_val & 0x7FFF) as u16;

    buf.put_u8(0x20 | (pts32_30 << 1) | 0x01); // '0010' + PTS[32..30] + marker
    buf.put_u8((pts29_15 >> 7) as u8);
    buf.put_u8(((pts29_15 & 0x7F) << 1) as u8 | 0x01);
    buf.put_u8((pts14_0 >> 7) as u8);
    buf.put_u8(((pts14_0 & 0x7F) << 1) as u8 | 0x01);

    // Payload (JPEG 数据)
    buf.put_slice(payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ps_header_length() {
        let mut buf = BytesMut::new();
        write_ps_header(&mut buf, 0);
        assert_eq!(buf.len(), 14, "PS Header 应为 14 字节");
    }

    #[test]
    fn test_system_header_length() {
        let mut buf = BytesMut::new();
        write_system_header(&mut buf);
        assert_eq!(buf.len(), 18, "System Header 应为 18 字节");
    }

    #[test]
    fn test_wrap_as_ps() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG SOI
        let ps = wrap_as_ps(&data, 0, "0000000001");
        assert!(ps.len() > 14);
        assert_eq!(&ps[0..4], &[0x00, 0x00, 0x01, 0xBA]); // PS start code
    }
}
