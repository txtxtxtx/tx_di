//! DBC 数据库解析与信号解码（MVP）
//!
//! 支持常见 DBC 子集：
//! - `BO_ <id> <name>: <dlc> <node>` 消息定义
//! - ` SG_ <name> : <start>|<len>@<order><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <node>` 信号定义
//! - 字节序：`1` = Intel(小端)，`0` = Motorola(大端)
//! - 符号：`+` 无符号，`-` 有符号
//!
//! 解码时按信号布局提取原始整数，再 `physical = raw * factor + offset`。

/// 字节序
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    /// 小端（Intel）
    Intel,
    /// 大端（Motorola）
    Motorola,
}

/// DBC 信号定义
#[derive(Debug, Clone)]
pub struct DbcSignal {
    pub name: String,
    /// 起始位（DBC 中的 start bit，按字节序解释）
    pub start_bit: u32,
    /// 信号位宽
    pub length: u32,
    pub byte_order: ByteOrder,
    pub is_signed: bool,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    pub unit: String,
}

/// DBC 消息定义
#[derive(Debug, Clone)]
pub struct DbcMessage {
    pub id: u32,
    pub name: String,
    pub dlc: u8,
    pub signals: Vec<DbcSignal>,
}

/// 解析后的 DBC 数据库
#[derive(Debug, Clone, Default)]
pub struct Dbc {
    pub messages: Vec<DbcMessage>,
}

impl Dbc {
    /// 解析 DBC 文本
    pub fn parse(s: &str) -> Result<Dbc, String> {
        let mut messages: Vec<DbcMessage> = Vec::new();
        let mut cur: Option<DbcMessage> = None;
        for line in s.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("BO_") {
                if let Some(m) = cur.take() {
                    messages.push(m);
                }
                // 256 MSG: 8 VCU
                let mut it = rest.split_whitespace();
                let id = it
                    .next()
                    .and_then(|v| u32::from_str_radix(v, 10).ok())
                    .ok_or("消息 ID 解析失败")?;
                let name = it
                    .next()
                    .ok_or("消息名缺失")?
                    .trim_end_matches(':')
                    .to_string();
                let dlc = it
                    .next()
                    .and_then(|v| v.parse::<u8>().ok())
                    .unwrap_or(8);
                cur = Some(DbcMessage {
                    id,
                    name,
                    dlc,
                    signals: Vec::new(),
                });
            } else if let Some(rest) = t.strip_prefix("SG_") {
                if let Some(m) = cur.as_mut() {
                    if let Some(sig) = parse_signal(rest) {
                        m.signals.push(sig);
                    }
                }
            }
        }
        if let Some(m) = cur.take() {
            messages.push(m);
        }
        Ok(Dbc { messages })
    }

    /// 按 CAN ID 查找消息
    pub fn message(&self, can_id: u32) -> Option<&DbcMessage> {
        self.messages.iter().find(|m| m.id == can_id)
    }

    /// 解码一帧：返回 (信号名, 物理量) 列表
    pub fn decode(&self, can_id: u32, data: &[u8]) -> Vec<(String, f64)> {
        let mut out = Vec::new();
        if let Some(msg) = self.message(can_id) {
            for sig in &msg.signals {
                let raw = extract_signal(data, sig);
                let raw = if sig.is_signed {
                    sign_extend(raw, sig.length) as f64
                } else {
                    raw as f64
                };
                let phys = raw * sig.factor + sig.offset;
                out.push((sig.name.clone(), phys));
            }
        }
        out
    }
}

/// 解析单个 `SG_` 行（去掉 `SG_` 前缀后）
fn parse_signal(s: &str) -> Option<DbcSignal> {
    // name : start|len@order± (factor,offset) [min|max] "unit" node
    let mut it = s.splitn(2, ':');
    let name = it.next()?.trim().to_string();
    let rest = it.next()?;
    // start|len@order± (factor,offset) [min|max] "unit" node
    let layout = rest.split_whitespace().next()?; // start|len@order+/-
    let (bitpart, signpart) = layout.split_once('@')?;
    let (start_s, len_s) = bitpart.split_once('|')?;
    let start_bit = start_s.trim().parse::<u32>().ok()?;
    let length = len_s.trim().parse::<u32>().unwrap_or(1);
    let order_ch = signpart.chars().next()?;
    let sign_ch = signpart.chars().nth(1).unwrap_or('+');
    let byte_order = if order_ch == '1' {
        ByteOrder::Intel
    } else {
        ByteOrder::Motorola
    };
    let is_signed = sign_ch == '-';

    // (factor,offset)
    let factor_off = rest.split('(').nth(1)?;
    let factor_off = factor_off.split(')').next()?;
    let mut fo = factor_off.split(',');
    let factor = fo.next()?.trim().parse::<f64>().unwrap_or(1.0);
    let offset = fo.next()?.trim().parse::<f64>().unwrap_or(0.0);

    // [min|max]
    let mut min = f64::MIN;
    let mut max = f64::MAX;
    if let Some(rng) = rest.split('[').nth(1) {
        if let Some(r) = rng.split(']').next() {
            let mut mm = r.split('|');
            if let Some(a) = mm.next().and_then(|v| v.trim().parse::<f64>().ok()) {
                min = a;
            }
            if let Some(b) = mm.next().and_then(|v| v.trim().parse::<f64>().ok()) {
                max = b;
            }
        }
    }

    // "unit"
    let unit = rest
        .split('"')
        .nth(1)
        .unwrap_or("")
        .to_string();

    Some(DbcSignal {
        name,
        start_bit,
        length,
        byte_order,
        is_signed,
        factor,
        offset,
        min,
        max,
        unit,
    })
}

/// 从字节数组提取信号原始无符号值（按字节序）
fn extract_signal(data: &[u8], sig: &DbcSignal) -> u64 {
    let mut raw: u64 = 0;
    for i in 0..sig.length {
        let bit = signal_bit_index(sig, i);
        let byte = (bit / 8) as usize;
        let bit_in_byte = (bit % 8) as u8;
        let val = if byte < data.len() {
            (data[byte] >> bit_in_byte) & 1
        } else {
            0
        };
        raw |= (val as u64) << i;
    }
    raw
}

/// 计算信号第 i 位（0-based，从 start 起）在字节数组中的绝对位序号
fn signal_bit_index(sig: &DbcSignal, i: u32) -> u32 {
    match sig.byte_order {
        ByteOrder::Intel => sig.start_bit + i,
        ByteOrder::Motorola => motorola_bit(sig.start_bit, i),
    }
}

/// Motorola（大端）位序：信号 LSB 位于 start_bit，向更高有效位移动时
/// 在字节内向更低的位号递进，跨字节边界则从上一字节的 bit7 继续。
fn motorola_bit(start: u32, k: u32) -> u32 {
    let mut b = (start / 8) as i32;
    let mut bit = (start % 8) as i32;
    for _ in 0..k {
        bit -= 1;
        if bit < 0 {
            bit = 7;
            b -= 1;
        }
    }
    (b * 8 + bit) as u32
}

/// 有符号位扩展
fn sign_extend(raw: u64, length: u32) -> i64 {
    if length == 0 || length >= 64 {
        return raw as i64;
    }
    let shift = 64 - length;
    (raw << shift) as i64 >> shift
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
BU_: VCU
BO_ 256 ENGINE_SPEED: 8 VCU
 SG_ RPM : 7|16@1+ (0.125,0) [0|8000] "rpm" VCU
 SG_ TEMP : 23|8@1- (1,-40) [-40|150] "degC" VCU
BO_ 260 BATTERY: 8 VCU
 SG_ VOLT : 12|12@0+ (0.01,0) [0|100] "V" VCU
"#;

    #[test]
    fn parse_basic() {
        let dbc = Dbc::parse(SAMPLE).unwrap();
        assert_eq!(dbc.messages.len(), 2);
        assert_eq!(dbc.message(256).unwrap().signals.len(), 2);
        assert_eq!(dbc.message(260).unwrap().signals.len(), 1);
    }

    #[test]
    fn decode_intel_unsigned() {
        let dbc = Dbc::parse(SAMPLE).unwrap();
        // RPM start=7 len=16 intel：信号最低位 = 全局 bit7
        // 仅置 bit7 (data[0]=0x80) -> raw=1 -> 0.125 rpm
        let data = [0x80, 0, 0, 0, 0, 0, 0, 0];
        let vals = dbc.decode(256, &data);
        let rpm = vals.iter().find(|(n, _)| n == "RPM").unwrap().1;
        assert!((rpm - 0.125).abs() < 1e-6, "rpm={rpm}");

        // 置全局 bit15 (data[1]=0x80) -> i=8 -> raw=256 -> 32 rpm
        let data2 = [0, 0x80, 0, 0, 0, 0, 0, 0];
        let vals2 = dbc.decode(256, &data2);
        let rpm2 = vals2.iter().find(|(n, _)| n == "RPM").unwrap().1;
        assert!((rpm2 - 32.0).abs() < 1e-6, "rpm2={rpm2}");
    }

    #[test]
    fn decode_motorola() {
        let dbc = Dbc::parse(SAMPLE).unwrap();
        let data = [0x34, 0x12, 0, 0, 0, 0, 0, 0];
        let vals = dbc.decode(260, &data);
        assert!(!vals.is_empty(), "Motorola 解码应至少返回一个信号");
        // 验证不会越界/溢出
        let v = vals.iter().find(|(n, _)| n == "VOLT").unwrap().1;
        assert!(v.is_finite());
    }
}
