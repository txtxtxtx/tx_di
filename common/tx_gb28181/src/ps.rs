//! 层次	关键组成	代码/值 (16进制)	说明
//!
//! PS 层 (最外层容器)	PS Header	起始码 00 00 01 BA	见下表详细字段说明
//!
//! System Header	起始码 00 00 01 BB	见下表详细字段说明
//!
//! PSM (Program Stream Map)	起始码 00 00 01 BC	见下表详细字段说明
//!
//! PES 层 (封装单个流)	Video PES Packet	起始码 00 00 01 + stream_id (如 E0)	封装视频帧数据
//!
//! Audio PES Packet (可选)	起始码 00 00 01 + stream_id (如 C0)	封装音频帧数据
//!
//! ES 层 (原始数据)	H.264 Raw Data	包含 SPS/PPS/IDR 的 NALU	视频帧数据
use deku::prelude::*;

/// 包起始码,固定值
pub const PACK_START_CODE: u32 = 0x000001BA;

/// PS 头
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")] // 全部字段大端
pub struct PsHeader {
    /// 包起始码 接收方只要看到 00 00 01 BA，就知道这是一个 PS 包的开始
    ///
    /// 32 位，固定值 0x000001BA
    #[deku(bits = "32")]
    pub pack_start_code: u32,  // 0x000001BA
    /// SCR 之前 有 2 位固定值 —— 必须是 01
    #[deku(bits = "2", assert_eq = "0b01")]
    pub marker_bits: u8, // 必须为 0b01
    /// 时间基准字段组 — 系统时钟参考 (SCR)
    // SCR_base[32..30] 3 bits
    #[deku(bits = "3")]
    pub scr_base_h: u8,
    // marker_bit = 1 (1 bit)
    #[deku(bits = "1", assert_eq = "1")]
    pub marker1: u8,
    #[deku(bits = "15")]
    pub scr_base_mid: u16,
    #[deku(bits = "1", assert_eq = "1")]
    pub marker2: u8,
    #[deku(bits = "15")]
    pub scr_base_low: u16,
    #[deku(bits = "1", assert_eq = "1")]
    pub marker3: u8,
    // SCR_extension (9 bits)
    #[deku(bits = "9")]
    pub scr_extension: u16,
    #[deku(bits = "1", assert_eq = "1")]
    pub marker4: u8,

    /// 节目复用速率
    // program_mux_rate (22 bits)
    /// 22 位无符号整数，单位是 50 字节/秒。
    ///
    /// 它告诉解码器该 PS 流可能达到的最大传输速率（用来分配缓冲区）。
    ///
    /// 标准缺省值：如果不做严格速率控制，可填全 1（即 0x3FFFFF）表示不限制。
    ///
    /// 同样跟随一个标记位 1
    #[deku(bits = "22")]
    pub program_mux_rate: u32,
    #[deku(bits = "1", assert_eq = "1")]
    pub marker5: u8,

    // reserved (5 bits)
    /// 5 位保留位，通常设为全 1（即 0x1F）
    #[deku(bits = "5")]
    pub reserved: u8,

    // pack_stuffing_length (3 bits)
    /// 填充长度
    #[deku(bits = "3")]
    pub pack_stuffing_length: u8,
}

/// 系统头起始码
pub const SYSTEM_HEADER_START_CODE: u32 = 0x000001BB;

/// System Header（系统头）
///
/// 仅在 **关键帧（IDR）PS 包** 中出现，位于 PS Header 之后、PSM 之前。
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct SystemHeader {
    // 系统头起始码
    #[deku(bits = "32", assert_eq = "0x000001BB")]
    pub system_header_start_code: u32,

    // header_length — 从下一字节到系统头结束的字节数（不含本长度字段）
    // 标准实现里一般采用固定值 12（0x0C）
    #[deku(bits = "16", assert_eq = "0x000C")]
    pub header_length: u16,

    // ---------- 以下为“系统参数”，共 12 字节 ----------
    // 第 6 字节：rate_bound 的高位（含 1 个 marker_bit）
    #[deku(bits = "8")]
    pub rate_bound_byte: u8,

    // 第 7‑8 字节：rate_bound 的剩余 22 位 + 1 个 marker_bit
    #[deku(bits = "16")]
    pub rate_bound_cont: u16,

    // 第 9 字节：audio_bound (6 bits) + 其他标志
    #[deku(bits = "8")]
    pub audio_bound_byte: u8,

    // 第 10 字节：video_bound (5 bits) + 其他标志
    #[deku(bits = "8")]
    pub video_bound_byte: u8,

    // 第 11‑17 字节：各种 rate_bound 标志、stream_id 等
    // 为简化并同时保证标准字节长度，我们将剩余 7 个字节
    // 作为一个整体保留，填入全 1。
    #[deku(bits = "56")]
    pub reserved: u64,
}

/// PSM 起始码
pub const PSM_START_CODE: u32 = 0x000001BC;

/// 流类型（stream_type）常量
pub mod stream_type {
    pub const H264:  u8 = 0x1B;
    pub const H265:  u8 = 0x24;
    pub const MPEG4: u8 = 0x10;
    pub const SVAC:  u8 = 0x80;
    pub const G711A: u8 = 0x90;
    pub const G711U: u8 = 0x91;
    pub const G7221: u8 = 0x92;
    pub const G7231: u8 = 0x93;
    pub const G729:  u8 = 0x99;
    pub const AAC:   u8 = 0x0F;
}

/// 基本流 ID（elementary_stream_id）常量
pub mod elementary_stream_id {
    pub const VIDEO: u8 = 0xE0;
    pub const AUDIO: u8 = 0xC0;
}

/// 基本流信息条目（描述一个基本流）
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big", ctx = "_: deku::ctx::Endian")]
pub struct ElementaryStreamInfo {
    /// 流类型（如 H.264 = 0x1B）
    pub stream_type: u8,
    /// 基本流 ID（视频 0xE0，音频 0xC0）
    pub elementary_stream_id: u8,
    /// 该基本流的描述信息长度（通常为 0x0000）
    pub info_length: u16,
}

/// Program Stream Map (PSM)
///
/// 仅在 **关键帧（IDR）PS 包** 中出现，位于 System Header 之后、Video PES 之前。
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct PSM {
    // PSM 起始码
    #[deku(bits = "32", assert_eq = "0x000001BC")]
    pub psm_start_code: u32,

    // PSM 长度（从下一字节到 CRC 前的字节数）
    // 这里由外部计算并填入，保证与实际字节流一致。
    #[deku(bits = "16")]
    pub psm_length: u16,

    // 当前/下一指示符 + 版本号（通常填 0xFF）
    #[deku(bits = "8")]
    pub version: u8,

    // 节目流信息长度（通常为 0x0000，表示无额外描述）
    #[deku(bits = "16")]
    pub program_stream_info_length: u16,

    // 基本流映射长度（紧随其后的所有 ElementaryStreamInfo 的总字节数）
    #[deku(bits = "16")]
    pub elementary_stream_map_length: u16,

    // 基本流信息列表
    #[deku(
        count = "elementary_stream_map_length / 4",
        // 保证每条 ElementaryStreamInfo 为 4 字节
        assert = "elementary_stream_map_length % 4 == 0"
    )]
    pub elementary_stream_infos: Vec<ElementaryStreamInfo>,

    // 注：GB/T 28181 实践中通常不需要 CRC_32，因此本文不再定义。
}

/// 视频 PES 起始码前缀
pub const PES_START_CODE_PREFIX: u32 = 0x000001;
/// 视频流 ID
pub const VIDEO_STREAM_ID: u8 = 0xE0;

/// Video PES Header（视频 PES 包头）
///
/// 包含 PTS，不含 DTS（GB/T 28181 要求 PTS/DTS 一致时可只传 PTS）
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct VideoPesHeader {
    // 包起始码前缀 (00 00 01)
    #[deku(bits = "24", assert_eq = "0x000001")]
    pub packet_start_code_prefix: u32,

    // 流 ID (视频固定 0xE0)
    #[deku(bits = "8", assert_eq = "0xE0")]
    pub stream_id: u8,

    // PES 包长度 (0 表示不限定长度)
    #[deku(bits = "16")]
    pub pes_packet_length: u16,

    // ---------- 可选 PES 包头 ----------
    // 前两个 bit 固定为 '10'
    #[deku(bits = "2", assert_eq = "0b10")]
    pub fixed_bits: u8,

    // PES_scrambling_control (2 bits) — 通常 00
    #[deku(bits = "2")]
    pub pes_scrambling_control: u8,

    // PES_priority (1 bit)
    #[deku(bits = "1")]
    pub pes_priority: u8,

    // data_alignment_indicator (1 bit)
    #[deku(bits = "1")]
    pub data_alignment_indicator: u8,

    // copyright (1 bit)
    #[deku(bits = "1")]
    pub copyright: u8,

    // original_or_copy (1 bit)
    #[deku(bits = "1")]
    pub original_or_copy: u8,

    // PTS_DTS_flags (2 bits) — 10 表示只有 PTS
    #[deku(bits = "2", assert_eq = "0b10")]
    pub pts_dts_flags: u8,

    // ESCR_flag (1 bit)
    #[deku(bits = "1")]
    pub escr_flag: u8,

    // ES_rate_flag (1 bit)
    #[deku(bits = "1")]
    pub es_rate_flag: u8,

    // DSM_trick_mode_flag (1 bit)
    #[deku(bits = "1")]
    pub dsm_trick_mode_flag: u8,

    // additional_copy_info_flag (1 bit)
    #[deku(bits = "1")]
    pub additional_copy_info_flag: u8,

    // PES_CRC_flag (1 bit)
    #[deku(bits = "1")]
    pub pes_crc_flag: u8,

    // PES_extension_flag (1 bit)
    #[deku(bits = "1")]
    pub pes_extension_flag: u8,

    // PES_header_data_length (8 bits) — 固定为 5，表示只有 PTS
    #[deku(bits = "8", assert_eq = "5")]
    pub pes_header_data_length: u8,

    // ---------- PTS (33 bits) ----------
    // 4 bit 固定 '0011' + PTS[32..30] 3 bits + marker_bit
    #[deku(bits = "8")]
    pub pts_byte1: u8,

    // PTS[29..22] + marker_bit
    #[deku(bits = "8")]
    pub pts_byte2: u8,

    // PTS[21..15] + marker_bit
    #[deku(bits = "8")]
    pub pts_byte3: u8,

    // PTS[14..7] + marker_bit
    #[deku(bits = "8")]
    pub pts_byte4: u8,

    // PTS[6..0] + marker_bit
    #[deku(bits = "8")]
    pub pts_byte5: u8,
}

/// 将 33 位 PTS 转为 5 字节 PES 时间戳字段
pub fn encode_pts_bytes(pts: u64) -> [u8; 5] {
    let pts = pts & 0x1FFFFFFFF; // 确保只有 33 位
    [
        // 第 1 字节：4 bit 0011 + PTS[32..30] + marker_bit(1)
        0b0011_0000
            | (((pts >> 30) & 0x07) as u8) << 1
            | 1u8,
        // 第 2 字节：PTS[29..22] + marker_bit(1)
        ((pts >> 22) & 0xFF) as u8,
        // 第 3 字节：PTS[21..15] + marker_bit(1)
        ((pts >> 15) & 0x7F) as u8,
        // 第 4 字节：PTS[14..7] + marker_bit(1)
        ((pts >> 7) & 0xFF) as u8,
        // 第 5 字节：PTS[6..0] + marker_bit(1)
        ((pts & 0x7F) as u8) << 1 | 1u8,
    ]
}

/// 将 33 位 DTS 转为 5 字节 PES 时间戳字段（同 PTS 编码规则）
pub fn encode_dts_bytes(dts: u64) -> [u8; 5] {
    encode_pts_bytes(dts)
}
