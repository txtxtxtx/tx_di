//! 固件文件解码（BIN / Motorola S-Record / Intel HEX）
//!
//! 供刷写引擎加载固件使用。解码结果为"段列表" `(起始地址, 数据)`，
//! 再线性化（按最低地址铺开，空隙填 0xFF），与 `FlashEngine` 的连续写入模型对齐。

use anyhow::{anyhow, Result};
use std::path::Path;

/// 单段：起始地址 + 数据
pub type Segment = (u32, Vec<u8>);

/// Motorola S-Record 解码
///
/// 支持 S0(头)/S1(16位)/S2(24位)/S3(32位) 数据记录，S5/S6(计数)、S7/S8/S9(起始地址) 忽略。
pub fn decode_s19(content: &str) -> Result<Vec<Segment>> {
    let mut segs = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('S') {
            continue;
        }
        let record_type = line.as_bytes().get(1).copied().unwrap_or(b'0');
        match record_type {
            b'0' | b'5' | b'6' | b'7' | b'8' | b'9' => continue, // 头/计数/起始地址，忽略
            b'1' | b'2' | b'3' => {}
            _ => return Err(anyhow!("S-Record 未知记录类型 S{} (行 {})", record_type as char, lineno + 1)),
        }

        // Sx <count><addr><data><checksum>，全部十六进制
        let bytes = hex_str_to_bytes(&line[2..])?;
        if bytes.len() < 2 {
            return Err(anyhow!("S-Record 数据过短 (行 {})", lineno + 1));
        }
        let count = bytes[0] as usize;
        if count == 0 || count + 1 > bytes.len() {
            return Err(anyhow!("S-Record count 非法 (行 {})", lineno + 1));
        }
        let addr_len = match record_type {
            b'1' => 2,
            b'2' => 3,
            _ => 4,
        };
        // count 包含 addr_len + data + checksum(1)
        let data_len = count - addr_len - 1;
        let mut addr = 0u32;
        for i in 0..addr_len {
            addr = (addr << 8) | bytes[1 + i] as u32;
        }
        let data = bytes[1 + addr_len..1 + addr_len + data_len].to_vec();
        segs.push((addr, data));
    }
    if segs.is_empty() {
        return Err(anyhow!("S-Record 未解析到任何数据段"));
    }
    Ok(segs)
}

/// Intel HEX 解码
///
/// 支持 00(数据)/01(EOF)/02(扩展段地址)/04(扩展线性地址)/03/05(起始地址，忽略)。
pub fn decode_intel_hex(content: &str) -> Result<Vec<Segment>> {
    let mut segs = Vec::new();
    let mut base_addr: u32 = 0; // 当前段/线性基址
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if !line.starts_with(':') {
            continue;
        }
        let bytes = hex_str_to_bytes(&line[1..])?;
        if bytes.len() < 5 {
            return Err(anyhow!("Intel HEX 记录过短 (行 {})", lineno + 1));
        }
        let len = bytes[0] as usize;
        let addr = ((bytes[1] as u32) << 8) | bytes[2] as u32;
        let rectype = bytes[3];
        let data = &bytes[4..4 + len];
        match rectype {
            0x00 => {
                segs.push((base_addr + addr, data.to_vec()));
            }
            0x01 => break, // EOF
            0x02 => {
                // 扩展段地址：base = (word) << 4
                if data.len() >= 2 {
                    base_addr = (((data[0] as u32) << 8) | data[1] as u32) << 4;
                }
            }
            0x04 => {
                // 扩展线性地址：base = (word) << 16
                if data.len() >= 2 {
                    base_addr = (((data[0] as u32) << 8) | data[1] as u32) << 16;
                }
            }
            0x03 | 0x05 => {} // 起始地址，忽略
            _ => return Err(anyhow!("Intel HEX 未知记录类型 0x{:02X} (行 {})", rectype, lineno + 1)),
        }
    }
    if segs.is_empty() {
        return Err(anyhow!("Intel HEX 未解析到任何数据段"));
    }
    Ok(segs)
}

/// 将段列表线性化为连续字节（从最低地址铺开，空隙填 0xFF）
pub fn segments_to_linear(segs: &[Segment]) -> Vec<u8> {
    if segs.is_empty() {
        return Vec::new();
    }
    let min_addr = segs.iter().map(|(a, _)| *a).min().unwrap();
    let max_end = segs
        .iter()
        .map(|(a, d)| a + d.len() as u32)
        .max()
        .unwrap();
    let mut buf = vec![0xFFu8; (max_end - min_addr) as usize];
    for (addr, data) in segs {
        let off = (*addr - min_addr) as usize;
        buf[off..off + data.len()].copy_from_slice(data);
    }
    buf
}

/// 根据扩展名自动选择解码器，返回线性固件字节
pub fn load_firmware(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let p = path.as_ref();
    let content = std::fs::read_to_string(p)
        .or_else(|_| std::fs::read(p).map(|b| String::from_utf8_lossy(&b).into_owned()))?;
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let segs = match ext.as_str() {
        "s19" | "s28" | "s37" | "srec" | "mot" => decode_s19(&content)?,
        "hex" | "ihx" | "ihex" => decode_intel_hex(&content)?,
        "bin" => {
            // 原始二进制：作为单段从地址 0 起
            return Ok(std::fs::read(p)?);
        }
        _ => {
            // 无法判断：尝试按 HEX，失败再按 S19
            decode_intel_hex(&content)
                .or_else(|_| decode_s19(&content))
                .map_err(|e| anyhow!("无法识别固件格式 ({}): {e}", p.display()))?
        }
    };
    Ok(segments_to_linear(&segs))
}

/// 将十六进制字符串（无空格）转为字节
fn hex_str_to_bytes(s: &str) -> Result<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(anyhow!("十六进制字符串长度必须为偶数"));
    }
    (0..s.len()).step_by(2).map(|i| {
        u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| anyhow!("十六进制解析失败: {e}"))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel_hex_decode() {
        let hex = ":100000000102030405060708090A0B0C0D0E0F10AE\n:00000001FF\n";
        let segs = decode_intel_hex(hex).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].0, 0x0000);
        assert_eq!(
            segs[0].1,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10]
        );
    }

    #[test]
    fn test_s19_decode() {
        // S1 记录：count=0x07(2 addr + 4 data + 1 checksum) addr=0x1234 data=4字节
        // checksum = ~(0x07+0x12+0x34+0x01+0x02+0x03+0x04) & 0xFF = 0xA8
        let s = "S10712340102030405A8\n";
        let segs = decode_s19(s).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].0, 0x1234);
        assert_eq!(segs[0].1, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_segments_to_linear_gap_filled() {
        let segs = vec![(0x00u32, vec![0xAA, 0xBB]), (0x04u32, vec![0xCC])];
        let lin = segments_to_linear(&segs);
        assert_eq!(lin, vec![0xAA, 0xBB, 0xFF, 0xFF, 0xCC]);
    }

    #[test]
    fn test_load_firmware_bin() {
        let dir = std::env::temp_dir();
        let p = dir.join("fw_bin_test.bin");
        std::fs::write(&p, vec![0x01, 0x02, 0x03]).unwrap();
        let data = load_firmware(&p).unwrap();
        assert_eq!(data, vec![0x01, 0x02, 0x03]);
        let _ = std::fs::remove_file(&p);
    }
}
