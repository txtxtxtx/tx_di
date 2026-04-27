//! CAN 帧数据结构

use serde::{Deserialize, Serialize};

/// CAN ID 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameId {
    /// 标准帧 11-bit (0x000..=0x7FF)
    Standard(u16),
    /// 扩展帧 29-bit (0x00000000..=0x1FFFFFFF)
    Extended(u32),
}

impl FrameId {
    /// 转为 u32 原始值
    pub fn raw(self) -> u32 {
        match self {
            FrameId::Standard(id) => id as u32,
            FrameId::Extended(id) => id,
        }
    }
    /// 是否扩展帧
    pub fn is_extended(self) -> bool {
        matches!(self, FrameId::Extended(_))
    }
    /// 从 u32 构造（>0x7FF 自动扩展帧）
    pub fn from_raw(id: u32) -> Self {
        if id > 0x7FF {
            FrameId::Extended(id)
        } else {
            FrameId::Standard(id as u16)
        }
    }
}

impl std::fmt::Display for FrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameId::Standard(id) => write!(f, "{:03X}", id),
            FrameId::Extended(id) => write!(f, "{:08X}", id),
        }
    }
}

/// 帧种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameKind {
    /// 标准数据帧
    Data,
    /// 远程帧
    Remote,
    /// 错误帧
    Error,
    /// CANFD 帧
    Fd,
}

/// CAN 标准帧（最大 8 字节）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanFrame {
    /// 帧 ID
    pub id: FrameId,
    /// 帧类型
    pub kind: FrameKind,
    /// 数据（最大 8 字节）
    pub data: Vec<u8>,
    /// 接收时间戳（µs，0 表示未记录）
    pub timestamp_us: u64,
}

impl CanFrame {
    /// 构造 CAN 帧（id 支持任意整数类型）
    pub fn new(id: impl TryInto<u32>, data: impl Into<Vec<u8>>) -> Self {
        let raw = id.try_into().unwrap_or(0);
        CanFrame {
            id: FrameId::from_raw(raw),
            kind: FrameKind::Data,
            data: data.into(),
            timestamp_us: 0,
        }
    }

    pub fn new_std(id: u16, data: impl Into<Vec<u8>>) -> Self {
        CanFrame {
            id: FrameId::Standard(id),
            kind: FrameKind::Data,
            data: data.into(),
            timestamp_us: 0,
        }
    }

    pub fn new_ext(id: u32, data: impl Into<Vec<u8>>) -> Self {
        CanFrame {
            id: FrameId::Extended(id),
            kind: FrameKind::Data,
            data: data.into(),
            timestamp_us: 0,
        }
    }

    /// DLC（数据长度码）
    pub fn dlc(&self) -> u8 {
        self.data.len().min(8) as u8
    }
}

/// CANFD 帧（最大 64 字节）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanFdFrame {
    /// 帧 ID
    pub id: FrameId,
    /// 数据（最大 64 字节）
    pub data: Vec<u8>,
    /// BRS (Bit Rate Switch)
    pub brs: bool,
    /// ESI (Error State Indicator)
    pub esi: bool,
    /// 接收时间戳（µs）
    pub timestamp_us: u64,
}

impl CanFdFrame {
    /// 构造 CANFD 帧（id 支持任意整数类型）
    pub fn new(id: impl TryInto<u32>, data: impl Into<Vec<u8>>) -> Self {
        let raw = id.try_into().unwrap_or(0);
        CanFdFrame {
            id: FrameId::from_raw(raw),
            data: data.into(),
            brs: true,
            esi: false,
            timestamp_us: 0,
        }
    }

    /// FD DLC（0..=15，按 ISO 11898-1:2015 映射到 0/1/.../8/12/16/20/24/32/48/64）
    pub fn fd_dlc(&self) -> u8 {
        let len = self.data.len();
        match len {
            0..=8 => len as u8,
            9..=12 => 9,
            13..=16 => 10,
            17..=20 => 11,
            21..=24 => 12,
            25..=32 => 13,
            33..=48 => 14,
            _ => 15,
        }
    }
}
