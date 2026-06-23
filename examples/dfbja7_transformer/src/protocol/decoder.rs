use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;
use crate::error::{AppError, AppResult};

/// 协议解码器
///
/// 协议格式：
/// +--------+--------+--------+--------+--------+
/// | 开始标志 | 长度   | 数据   | RSSI   | CRC    |
/// +--------+--------+--------+--------+--------+
/// | 1字节   | 1字节  | 变长   | 1字节  | 2字节  |
/// +--------+--------+--------+--------+--------+
///
/// 开始标志: 0x55 (上行) 或 0xAA (下行)
/// 长度: 压缩长度，>128时使用特殊编码
/// CRC: CRC32/MPEG-2 校验码
pub struct DeviceCodec {
    /// 最大帧长度
    max_frame_length: usize,
    /// 当前状态
    state: DecodeState,
    /// 缓冲区
    buffer: BytesMut,
}

/// 解码状态
#[derive(Debug, Clone)]
enum DecodeState {
    /// 等待开始标志
    WaitingForStart,
    /// 等待长度字节
    WaitingForLength,
    /// 等待完整帧
    WaitingForFrame(usize),
}

impl DeviceCodec {
    /// 创建新的解码器
    pub fn new(max_frame_length: usize) -> Self {
        DeviceCodec {
            max_frame_length,
            state: DecodeState::WaitingForStart,
            buffer: BytesMut::with_capacity(max_frame_length),
        }
    }

    /// 创建默认解码器（最大帧长度382字节）
    pub fn default_codec() -> Self {
        Self::new(382)
    }

    /// 解码单个帧
    fn decode_frame(&mut self, src: &mut BytesMut) -> AppResult<Option<String>> {
        // 检查是否有足够的数据
        if src.len() < 2 {
            return Ok(None);
        }

        // 检查开始标志
        let start_byte = src[0];
        if start_byte != 0x55 && start_byte != 0xAA {
            // 跳过无效字节
            src.advance(1);
            return Ok(None);
        }

        // 获取长度字节
        let length_byte = src[1];
        let frame_length = self.calculate_frame_length(length_byte)?;

        // 检查帧长度是否合理
        if frame_length > self.max_frame_length {
            return Err(AppError::Protocol(format!(
                "帧长度超过最大限制: {} > {}",
                frame_length, self.max_frame_length
            )));
        }

        // 检查是否有足够的数据
        if src.len() < frame_length {
            return Ok(None);
        }

        // 提取帧数据
        let frame_data = src.split_to(frame_length);
        let frame_hex = crate::util::convert::bytes_to_hex(&frame_data);

        Ok(Some(frame_hex))
    }

    /// 计算帧长度
    fn calculate_frame_length(&self, length_byte: u8) -> AppResult<usize> {
        let actual_length = if length_byte > 128 {
            // 压缩长度: (length_byte - 128) / 2 + 128
            ((length_byte - 128) as usize) / 2 + 128
        } else {
            length_byte as usize
        };

        // 加上开始标志(1字节)、长度字节(1字节)、RSSI(1字节)、CRC(2字节)
        Ok(actual_length + 5)
    }
}

impl Decoder for DeviceCodec {
    type Item = String;
    type Error = AppError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 循环尝试解码
        loop {
            match &self.state {
                DecodeState::WaitingForStart => {
                    if src.is_empty() {
                        return Ok(None);
                    }

                    // 查找开始标志
                    if let Some(pos) = src.iter().position(|&b| b == 0x55 || b == 0xAA) {
                        // 跳过开始标志之前的数据
                        if pos > 0 {
                            src.advance(pos);
                        }
                        self.state = DecodeState::WaitingForLength;
                    } else {
                        // 没有找到开始标志，清空缓冲区
                        src.clear();
                        return Ok(None);
                    }
                }
                DecodeState::WaitingForLength => {
                    if src.len() < 2 {
                        return Ok(None);
                    }

                    let length_byte = src[1];
                    let frame_length = self.calculate_frame_length(length_byte)?;
                    self.state = DecodeState::WaitingForFrame(frame_length);
                }
                DecodeState::WaitingForFrame(frame_length) => {
                    if src.len() < *frame_length {
                        return Ok(None);
                    }

                    // 提取帧
                    let frame_data = src.split_to(*frame_length);
                    let frame_hex = crate::util::convert::bytes_to_hex(&frame_data);

                    // 重置状态
                    self.state = DecodeState::WaitingForStart;

                    return Ok(Some(frame_hex));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_decode_frame() {
        let mut codec = DeviceCodec::default_codec();
        let mut src = BytesMut::new();

        // 测试报文: 552E040D2D42D723000100000000000000D2351A1542AD78EC420C06000000000000000000003C000000000EA120
        // 长度字节 0x2E = 46，所以总帧长度 = 46 + 5 = 51 字节
        let test_data = vec![
            0x55, 0x2E, 0x04, 0x0D, 0x2D, 0x42, 0xD7, 0x23,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0xD2, 0x35, 0x1A, 0x15, 0x42, 0xAD, 0x78,
            0xEC, 0x42, 0x0C, 0x06, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x00,
            0x00, 0x00, 0x00, 0x0E, 0xA1, 0x20, 0x00, 0x00,
            0x00, 0x00, 0x00,
        ];

        src.extend_from_slice(&test_data);

        let result = codec.decode(&mut src).unwrap();
        assert!(result.is_some());
    }
}