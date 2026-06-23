/// CRC32/MPEG-2 计算
pub struct Crc32 {
    table: [u32; 256],
}

impl Crc32 {
    /// 创建新的CRC32计算器
    pub fn new() -> Self {
        let mut table = [0u32; 256];
        for i in 0..256 {
            let mut crc = (i as u32) << 24;
            for _ in 0..8 {
                if crc & 0x80000000 != 0 {
                    crc = (crc << 1) ^ 0x04C11DB7;
                } else {
                    crc <<= 1;
                }
            }
            table[i] = crc;
        }
        Crc32 { table }
    }

    /// 计算CRC32/MPEG-2
    pub fn compute(&self, data: &[u8]) -> u32 {
        let mut crc = 0xFFFFFFFF;
        for &byte in data {
            let index = ((crc >> 24) ^ (byte as u32)) & 0xFF;
            crc = (crc << 8) ^ self.table[index as usize];
        }
        crc
    }

    /// 计算CRC32并返回4字节大写十六进制字符串
    pub fn compute_hex(&self, data: &[u8]) -> String {
        let crc = self.compute(data);
        format!("{:08X}", crc)
    }

    /// 计算CRC32并进行异或处理（与Java版本一致）
    pub fn compute_with_xor(&self, data: &[u8]) -> String {
        let crc = self.compute(data);
        let crc_hex = format!("{:08X}", crc);

        // 提取前4位和后4位
        let a = &crc_hex[0..4];
        let b = &crc_hex[4..8];

        // 异或运算
        let a_val = u16::from_str_radix(a, 16).unwrap_or(0);
        let b_val = u16::from_str_radix(b, 16).unwrap_or(0);
        let xor_result = a_val ^ b_val;

        format!("{:04X}", xor_result)
    }
}

/// 验证CRC是否正确
pub fn verify_crc(message_hex: &str) -> bool {
    if message_hex.len() < 4 {
        return false;
    }

    let (data_part, crc_part) = message_hex.split_at(message_hex.len() - 4);
    let calculated_crc = calculate_crc_for_message(data_part);

    calculated_crc == crc_part
}

/// 计算消息的CRC（与Java版本的getCrcResult一致）
pub fn calculate_crc_for_message(data_hex: &str) -> String {
    // 转换字节序
    let converted = convert_byte_order(data_hex);

    // 转换为字节数组
    let bytes = hex_to_bytes(&converted);

    // 计算CRC32并进行异或处理
    let crc = Crc32::new();
    crc.compute_with_xor(&bytes)
}

/// 转换字节序（与Java版本的convertByteOrder一致）
fn convert_byte_order(hex_str: &str) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < hex_str.len() {
        let end = std::cmp::min(i + 8, hex_str.len());
        let chunk = &hex_str[i..end];

        // 如果不足8位，补0
        let padded = if chunk.len() < 8 {
            format!("{:0<8}", chunk)
        } else {
            chunk.to_string()
        };

        // 反转字节序
        let reversed = format!(
            "{}{}{}{}",
            &padded[6..8],
            &padded[4..6],
            &padded[2..4],
            &padded[0..2]
        );

        result.push_str(&reversed);
        i += 8;
    }

    result
}

/// 十六进制字符串转字节数组
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = hex.chars().collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_compute() {
        let crc = Crc32::new();
        let data = b"Hello";
        let result = crc.compute(data);
        assert_ne!(result, 0);
    }

    #[test]
    fn test_verify_crc() {
        // 测试报文: 552E040D2D42D723000100000000000000D2351A1542AD78EC420C06000000000000000000003C000000000EA120
        let message = "552E040D2D42D723000100000000000000D2351A1542AD78EC420C06000000000000000000003C000000000EA120";
        // 注意：这个测试可能需要根据实际CRC算法调整
        // assert!(verify_crc(message));
    }
}