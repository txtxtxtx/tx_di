use crate::error::{AppError, AppResult};

/// 基础协议结构
///
/// 协议格式：
/// +--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+
/// | feed   | length | payload| uuid   | subLen | cmd    | ver    | data   | id     | rssi   | crc    |
/// +--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+
/// | 1字节  | 1字节  | 1字节  | 4字节  | 1字节  | 1字节  | 1字节  | 变长   | 1字节  | 1字节  | 2字节  |
/// +--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+
#[derive(Debug, Clone)]
pub struct BaseMessage {
    /// 报文类型: 0x55 (上行) 或 0xAA (下行)
    pub feed: u8,
    /// 报文长度 (压缩后)
    pub length_zip: u8,
    /// 设备协议号 (类型)
    pub payload_id: u8,
    /// 设备唯一码
    pub uuid: u32,
    /// 子数据包字节数
    pub sub_length_zip: u8,
    /// 命令码
    pub cmd: u8,
    /// 版本码
    pub ver: u8,
    /// 数据部分 (十六进制字符串)
    pub data: String,
    /// 同步码
    pub id: u8,
    /// 无线信号强度
    pub rssi: u8,
    /// CRC校验码
    pub crc: u16,
}

impl BaseMessage {
    /// 从十六进制字符串解析基础消息
    pub fn from_hex(hex: &str) -> AppResult<Self> {
        if hex.len() < 20 {
            return Err(AppError::Protocol("消息长度不足".to_string()));
        }

        // 解析各个字段
        let feed = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| AppError::Protocol("解析feed失败".to_string()))?;

        let length_zip = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| AppError::Protocol("解析length_zip失败".to_string()))?;

        let payload_id = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| AppError::Protocol("解析payload_id失败".to_string()))?;

        let uuid = u32::from_str_radix(&hex[6..14], 16)
            .map_err(|_| AppError::Protocol("解析uuid失败".to_string()))?;

        let sub_length_zip = u8::from_str_radix(&hex[14..16], 16)
            .map_err(|_| AppError::Protocol("解析sub_length_zip失败".to_string()))?;

        let cmd = u8::from_str_radix(&hex[16..18], 16)
            .map_err(|_| AppError::Protocol("解析cmd失败".to_string()))?;

        let ver = u8::from_str_radix(&hex[18..20], 16)
            .map_err(|_| AppError::Protocol("解析ver失败".to_string()))?;

        // 数据部分: 从第20个字符到倒数第8个字符
        let data_end = hex.len() - 8;
        if data_end < 20 {
            return Err(AppError::Protocol("数据部分长度不足".to_string()));
        }
        let data = hex[20..data_end].to_string();

        // 解析尾部字段
        let tail_start = hex.len() - 8;
        let id = u8::from_str_radix(&hex[tail_start..tail_start + 2], 16)
            .map_err(|_| AppError::Protocol("解析id失败".to_string()))?;

        let rssi = u8::from_str_radix(&hex[tail_start + 2..tail_start + 4], 16)
            .map_err(|_| AppError::Protocol("解析rssi失败".to_string()))?;

        let crc = u16::from_str_radix(&hex[tail_start + 4..tail_start + 8], 16)
            .map_err(|_| AppError::Protocol("解析crc失败".to_string()))?;

        Ok(BaseMessage {
            feed,
            length_zip,
            payload_id,
            uuid,
            sub_length_zip,
            cmd,
            ver,
            data,
            id,
            rssi,
            crc,
        })
    }

    /// 获取设备类型标识
    pub fn get_template_id(&self) -> String {
        format!("{:02X}_{:02X}_{:02X}", self.payload_id, self.sub_length_zip, self.ver)
    }

    /// 获取设备唯一码字符串
    pub fn get_uuid_string(&self) -> String {
        self.uuid.to_string()
    }

    /// 获取RSSI描述
    pub fn get_rssi_description(&self) -> String {
        match self.rssi {
            0 => "-113dBm or less".to_string(),
            1..=30 => {
                let value = -111 + (self.rssi as i32 - 1) * 2;
                format!("{}dBm", value)
            }
            31 => "-51dBm or greater".to_string(),
            _ => "No network or undetectable".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_message_parse() {
        // 测试报文: 552E040D2D42D723000100000000000000D2351A1542AD78EC420C06000000000000000000003C000000000EA120
        let hex = "552E040D2D42D723000100000000000000D2351A1542AD78EC420C06000000000000000000003C000000000EA120";
        let message = BaseMessage::from_hex(hex).unwrap();

        assert_eq!(message.feed, 0x55);
        assert_eq!(message.length_zip, 0x2E);
        assert_eq!(message.payload_id, 0x04);
        assert_eq!(message.uuid, 0x0D2D42D7);
        assert_eq!(message.sub_length_zip, 0x23);
        assert_eq!(message.cmd, 0x00);
        assert_eq!(message.ver, 0x01);
    }

    #[test]
    fn test_get_template_id() {
        let hex = "552E040D2D42D723000100000000000000D2351A1542AD78EC420C06000000000000000000003C000000000EA120";
        let message = BaseMessage::from_hex(hex).unwrap();
        assert_eq!(message.get_template_id(), "04_23_01");
    }
}