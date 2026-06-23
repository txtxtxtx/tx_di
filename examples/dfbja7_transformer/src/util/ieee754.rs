/// IEEE754浮点数转换工具
pub struct Ieee754;

impl Ieee754 {
    /// 将4字节大端序字节数组转换为f32（IEEE754标准）
    pub fn bytes_to_float(bytes: &[u8; 4]) -> f32 {
        let bits = u32::from_be_bytes(*bytes);
        f32::from_bits(bits)
    }

    /// 将8字节大端序字节数组转换为f64（IEEE754标准）
    pub fn bytes_to_double(bytes: &[u8; 8]) -> f64 {
        let bits = u64::from_be_bytes(*bytes);
        f64::from_bits(bits)
    }

    /// 将十六进制字符串转换为f32
    pub fn hex_str_to_float(hex_str: &str) -> f32 {
        let bytes = hex_str_to_bytes(hex_str);
        if bytes.len() >= 4 {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&bytes[0..4]);
            Self::bytes_to_float(&buf)
        } else {
            0.0
        }
    }

    /// 将十六进制字符串转换为f64
    pub fn hex_str_to_double(hex_str: &str) -> f64 {
        let bytes = hex_str_to_bytes(hex_str);
        if bytes.len() >= 8 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[0..8]);
            Self::bytes_to_double(&buf)
        } else {
            0.0
        }
    }

    /// 将f32转换为4字节大端序字节数组
    pub fn float_to_bytes(value: f32) -> [u8; 4] {
        value.to_be_bytes()
    }

    /// 将f64转换为8字节大端序字节数组
    pub fn double_to_bytes(value: f64) -> [u8; 8] {
        value.to_be_bytes()
    }

    /// 将f32转换为十六进制字符串
    pub fn float_to_hex_str(value: f32) -> String {
        let bytes = Self::float_to_bytes(value);
        bytes_to_hex_str(&bytes)
    }

    /// 将f64转换为十六进制字符串
    pub fn double_to_hex_str(value: f64) -> String {
        let bytes = Self::double_to_bytes(value);
        bytes_to_hex_str(&bytes)
    }
}

/// 十六进制字符串转字节数组
fn hex_str_to_bytes(hex_str: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let chars: Vec<char> = hex_str.chars().collect();
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
fn bytes_to_hex_str(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

/// 从8字节十六进制字符串中提取经纬度（与Java版本一致）
pub fn extract_gps_coordinates(gps_hex: &str) -> (f32, f32) {
    if gps_hex.len() < 16 {
        return (0.0, 0.0);
    }

    let mid = gps_hex.len() / 2;
    let longitude_hex = &gps_hex[mid..];
    let latitude_hex = &gps_hex[..mid];

    let longitude = Ieee754::hex_str_to_float(longitude_hex);
    let latitude = Ieee754::hex_str_to_float(latitude_hex);

    (longitude, latitude)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_str_to_float() {
        // 测试IEEE754转换
        let hex = "41200000"; // 10.0 in IEEE754
        let value = Ieee754::hex_str_to_float(hex);
        assert!((value - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_float_to_hex_str() {
        let value = 10.0f32;
        let hex = Ieee754::float_to_hex_str(value);
        assert_eq!(hex, "41200000");
    }

    #[test]
    fn test_extract_gps_coordinates() {
        // 测试GPS坐标提取
        let gps_hex = "4120000041200000"; // 两个10.0
        let (lng, lat) = extract_gps_coordinates(gps_hex);
        assert!((lng - 10.0).abs() < 0.001);
        assert!((lat - 10.0).abs() < 0.001);
    }
}