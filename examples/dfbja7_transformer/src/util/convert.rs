/// 转换工具模块

/// 十六进制字符串转字节数组
pub fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let chars: Vec<char> = hex.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let high = chars[i].to_digit(16).unwrap_or(0) as u8;
        let low = if i + 1 < chars.len() {
            chars[i + 1].to_digit(16).unwrap_or(0) as u8
        } else {
            0
        };
        bytes.push((high << 4) | low);
        i += 2;
    }

    bytes
}

/// 字节数组转十六进制字符串
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

/// 十六进制字符串转u8
pub fn hex_to_u8(hex: &str) -> u8 {
    u8::from_str_radix(hex, 16).unwrap_or(0)
}

/// 十六进制字符串转u16
pub fn hex_to_u16(hex: &str) -> u16 {
    u16::from_str_radix(hex, 16).unwrap_or(0)
}

/// 十六进制字符串转u32
pub fn hex_to_u32(hex: &str) -> u32 {
    u32::from_str_radix(hex, 16).unwrap_or(0)
}

/// 十六进制字符串转i32
pub fn hex_to_i32(hex: &str) -> i32 {
    i32::from_str_radix(hex, 16).unwrap_or(0)
}

/// 十进制转十六进制字符串（2位）
pub fn decimal_to_hex(dec: u8) -> String {
    format!("{:02X}", dec)
}

/// 十进制转二进制字符串（指定位数）
pub fn decimal_to_binary(num: u32, size: usize) -> String {
    format!("{:0>width$b}", num, width = size)
}

/// 十六进制字符串转二进制数组（8位）
pub fn hex_to_binary_array(hex: &str) -> Vec<String> {
    let value = u8::from_str_radix(hex, 16).unwrap_or(0);
    let binary_str = decimal_to_binary(value as u32, 8);

    binary_str
        .chars()
        .collect::<Vec<char>>()
        .chunks(2)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

/// 十六进制字符串转报警级别数组
pub fn hex_to_alarm_level_array(hex: &str) -> Vec<u8> {
    let binary_array = hex_to_binary_array(hex);
    binary_array
        .iter()
        .map(|s| u8::from_str_radix(s, 2).unwrap_or(0))
        .collect()
}

/// 十六进制字符串转特殊报警数组
pub fn hex_to_alarm_sp_array(hex: &str) -> Vec<usize> {
    let value = u8::from_str_radix(hex, 16).unwrap_or(0);
    let binary_str = decimal_to_binary(value as u32, 8);

    binary_str
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == '1')
        .map(|(i, _)| i)
        .collect()
}

/// 获取气体值（处理特殊值）
pub fn get_gas_decimal(hex: &str) -> i32 {
    match hex {
        "FF00" | "8000" => -1, // 屏蔽该路
        "FFFF" => -2,          // 故障
        _ => i32::from_str_radix(hex, 16).unwrap_or(0),
    }
}

/// 根据小数位数格式化数值
pub fn format_decimal_value(value: i32, dot: Option<u8>) -> String {
    match dot {
        None => value.to_string(),
        Some(0) => {
            if value == -2 {
                "fault".to_string()
            } else {
                value.to_string()
            }
        }
        Some(d) => {
            if value == -2 {
                "fault".to_string()
            } else {
                let divisor = 10_i32.pow(d as u32);
                let integer_part = value / divisor;
                let decimal_part = (value % divisor).abs();
                format!("{}.{:0>width$}", integer_part, decimal_part, width = d as usize)
            }
        }
    }
}

/// 获取RSSI值描述
pub fn get_rssi_value(rssi_hex: &str) -> String {
    let rssi = u8::from_str_radix(rssi_hex, 16).unwrap_or(0);
    match rssi {
        0 => "-113dBm or less".to_string(),
        1..=30 => {
            let value = -111 + (rssi as i32 - 1) * 2;
            format!("{}dBm", value)
        }
        31 => "-51dBm or greater".to_string(),
        _ => "No network or undetectable".to_string(),
    }
}

/// 获取模板ID（设备类型标识）
pub fn get_template_id(message: &str) -> anyhow::Result<String> {
    if message.len() < 20 {
        return Err(anyhow::anyhow!("消息长度不足"));
    }

    let payload_id = &message[4..6];
    let sub_length_zip = &message[14..16];
    let ver = &message[18..20];

    Ok(format!("{}_{}_{}", payload_id, sub_length_zip, ver))
}

/// 验证消息长度
pub fn verify_message_length(message: &str) -> bool {
    if message.len() < 4 {
        return false;
    }

    let length_zip_hex = &message[2..4];
    let length_zip = u8::from_str_radix(length_zip_hex, 16).unwrap_or(0);

    // 处理长度压缩逻辑
    let actual_length = if length_zip > 128 {
        ((length_zip - 128) / 2 + 128) as usize
    } else {
        length_zip as usize
    };

    let byte_count = message.len() / 2;
    actual_length == byte_count
}

/// 验证消息CRC
pub fn verify_message_crc(message: &str) -> bool {
    if message.len() < 4 {
        return false;
    }

    let data_part = &message[..message.len() - 4];
    let crc_part = &message[message.len() - 4..];

    let calculated_crc = crate::util::crc32::calculate_crc_for_message(data_part);
    calculated_crc == crc_part
}

/// 验证消息完整性
pub fn verify_message(message: &str) -> bool {
    verify_message_length(message) && verify_message_crc(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        let hex = "552E04";
        let bytes = hex_to_bytes(hex);
        assert_eq!(bytes, vec![0x55, 0x2E, 0x04]);
    }

    #[test]
    fn test_bytes_to_hex() {
        let bytes = vec![0x55, 0x2E, 0x04];
        let hex = bytes_to_hex(&bytes);
        assert_eq!(hex, "552E04");
    }

    #[test]
    fn test_hex_to_u8() {
        assert_eq!(hex_to_u8("55"), 0x55);
        assert_eq!(hex_to_u8("FF"), 0xFF);
    }

    #[test]
    fn test_decimal_to_hex() {
        assert_eq!(decimal_to_hex(0x55), "55");
        assert_eq!(decimal_to_hex(0xFF), "FF");
    }

    #[test]
    fn test_get_gas_decimal() {
        assert_eq!(get_gas_decimal("FF00"), -1);
        assert_eq!(get_gas_decimal("8000"), -1);
        assert_eq!(get_gas_decimal("FFFF"), -2);
        assert_eq!(get_gas_decimal("0064"), 100);
    }

    #[test]
    fn test_format_decimal_value() {
        assert_eq!(format_decimal_value(100, None), "100");
        assert_eq!(format_decimal_value(100, Some(0)), "100");
        assert_eq!(format_decimal_value(100, Some(1)), "10.0");
        assert_eq!(format_decimal_value(1234, Some(2)), "12.34");
        assert_eq!(format_decimal_value(-2, Some(1)), "fault");
    }

    #[test]
    fn test_get_rssi_value() {
        assert_eq!(get_rssi_value("00"), "-113dBm or less");
        assert_eq!(get_rssi_value("01"), "-111dBm");
        assert_eq!(get_rssi_value("1F"), "-51dBm or greater");
    }

    #[test]
    fn test_hex_to_alarm_level_array() {
        let result = hex_to_alarm_level_array("40");
        assert_eq!(result, vec![1, 0, 0, 0]);
    }

    #[test]
    fn test_hex_to_alarm_sp_array() {
        let result = hex_to_alarm_sp_array("02");
        assert_eq!(result, vec![6]);
    }
}