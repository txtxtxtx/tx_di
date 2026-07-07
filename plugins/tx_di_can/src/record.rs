//! 总线录制与回放（CSV，MVP）
//!
//! 录制：将接收到的 CAN / CAN-FD 帧写入 CSV（时间、类别、ID、DLC、数据）。
//! 回放：读取 CSV，按原始时间间隔（可调速度）重新发送到总线，用于问题复现。
//! 后续可扩展 BLF/ASC 格式。

use crate::adapter::CanAdapter;
use crate::frame::{CanFdFrame, CanFrame, FrameId};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::time::Duration;

/// 单条录制记录
#[derive(Debug, Clone)]
pub struct FrameRecord {
    pub timestamp_us: u64,
    pub is_fd: bool,
    pub id: u32,
    pub brs: bool,
    pub esi: bool,
    pub data: Vec<u8>,
}

impl FrameRecord {
    pub fn from_can(f: &CanFrame) -> Self {
        FrameRecord {
            timestamp_us: f.timestamp_us,
            is_fd: false,
            id: f.id.raw(),
            brs: false,
            esi: false,
            data: f.data.clone(),
        }
    }

    pub fn from_fd(f: &CanFdFrame) -> Self {
        FrameRecord {
            timestamp_us: f.timestamp_us,
            is_fd: true,
            id: f.id.raw(),
            brs: f.brs,
            esi: f.esi,
            data: f.data.clone(),
        }
    }

    /// 序列化为 CSV 行
    pub fn to_csv(&self) -> String {
        let kind = if self.is_fd { 'F' } else { 'C' };
        let brs = if self.brs { '1' } else { '0' };
        let esi = if self.esi { '1' } else { '0' };
        let data = self
            .data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{},{},{:X},{},{},{},{}\n",
            self.timestamp_us, kind, self.id, brs, esi, self.data.len(), data
        )
    }

    /// 从 CSV 行解析（容错：缺少字段则忽略该行）
    pub fn from_csv(line: &str) -> Option<FrameRecord> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 7 {
            return None;
        }
        let timestamp_us = parts[0].trim().parse().ok()?;
        let is_fd = parts[1].trim() == "F";
        let id = u32::from_str_radix(parts[2].trim(), 16).ok()?;
        let brs = parts[3].trim() == "1";
        let esi = parts[4].trim() == "1";
        // parts[5] = dlc（数据长度，可选）
        let data = parts[6..]
            .join(",")
            .split_whitespace()
            .filter_map(|h| u8::from_str_radix(h, 16).ok())
            .collect::<Vec<_>>();
        Some(FrameRecord {
            timestamp_us,
            is_fd,
            id,
            brs,
            esi,
            data,
        })
    }
}

/// CSV 录制器（追加写入）
pub struct Recorder {
    file: std::fs::File,
}

impl Recorder {
    /// 新建录制器并写入表头
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = std::fs::File::create(path)?;
        use std::io::Write;
        file.write_all(b"timestamp_us,type,id,brs,esi,dlc,data\n")?;
        Ok(Recorder { file })
    }

    /// 记录一帧
    pub fn record(&mut self, rec: &FrameRecord) -> Result<()> {
        use std::io::Write;
        self.file.write_all(rec.to_csv().as_bytes())?;
        Ok(())
    }

    pub fn record_can(&mut self, f: &CanFrame) -> Result<()> {
        self.record(&FrameRecord::from_can(f))
    }

    pub fn record_fd(&mut self, f: &CanFdFrame) -> Result<()> {
        self.record(&FrameRecord::from_fd(f))
    }
}

/// 读取 CSV 全部记录
pub fn load_csv(path: impl AsRef<Path>) -> Result<Vec<FrameRecord>> {
    let content = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines().skip(1) {
        if let Some(rec) = FrameRecord::from_csv(line) {
            out.push(rec);
        }
    }
    Ok(out)
}

/// 回放 CSV 到总线（按原始间隔 × speed_factor 倍速；speed_factor<1 更快）
///
/// `speed_factor`：0.5 两倍速，2.0 半速；0 表示不等待（尽量快）。
pub async fn replay_csv(
    path: impl AsRef<Path>,
    adapter: &dyn CanAdapter,
    speed_factor: f64,
) -> Result<usize> {
    let records = load_csv(path)?;
    if records.is_empty() {
        return Ok(0);
    }
    let mut sent = 0usize;
    let mut last_ts = records[0].timestamp_us;
    for rec in &records {
        if speed_factor > 0.0 && rec.timestamp_us > last_ts {
            let delta_ms = (rec.timestamp_us - last_ts) as f64 / 1000.0 * speed_factor;
            if delta_ms > 0.0 {
                tokio::time::sleep(Duration::from_secs_f64(delta_ms)).await;
            }
        }
        last_ts = rec.timestamp_us;
        if rec.is_fd {
            let frame = CanFdFrame {
                id: FrameId::from_raw(rec.id),
                data: rec.data.clone(),
                brs: rec.brs,
                esi: rec.esi,
                timestamp_us: 0,
            };
            adapter.send_fd(&frame).await?;
        } else {
            let frame = CanFrame {
                id: FrameId::from_raw(rec.id),
                kind: crate::frame::FrameKind::Data,
                data: rec.data.clone(),
                timestamp_us: 0,
            };
            adapter.send(&frame).await?;
        }
        sent += 1;
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::SimBusAdapter;

    #[test]
    fn test_csv_roundtrip() {
        let rec = FrameRecord {
            timestamp_us: 123456,
            is_fd: false,
            id: 0x7E8,
            brs: false,
            esi: false,
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let line = rec.to_csv();
        let parsed = FrameRecord::from_csv(&line).unwrap();
        assert_eq!(parsed.timestamp_us, 123456);
        assert_eq!(parsed.id, 0x7E8);
        assert_eq!(parsed.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_csv_fd_roundtrip() {
        let rec = FrameRecord {
            timestamp_us: 999,
            is_fd: true,
            id: 0x100,
            brs: true,
            esi: false,
            data: vec![0x11, 0x22, 0x33],
        };
        let parsed = FrameRecord::from_csv(&rec.to_csv()).unwrap();
        assert!(parsed.is_fd);
        assert!(parsed.brs);
        assert_eq!(parsed.data, vec![0x11, 0x22, 0x33]);
    }

    #[tokio::test]
    async fn test_replay_csv() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("tx_di_can_replay_{}.csv", std::process::id()));
        {
            let mut rec = Recorder::new(&p).unwrap();
            rec.record(&FrameRecord {
                timestamp_us: 1000,
                is_fd: false,
                id: 0x7E0,
                brs: false,
                esi: false,
                data: vec![0x22, 0xF1, 0x90],
            })
            .unwrap();
            rec.record(&FrameRecord {
                timestamp_us: 2000,
                is_fd: false,
                id: 0x7E1,
                brs: false,
                esi: false,
                data: vec![0x01, 0x02],
            })
            .unwrap();
        }
        let adapter = SimBusAdapter::new("replay", 64);
        adapter.open().await.unwrap();
        let mut rx = adapter.subscribe();
        let sent = replay_csv(&p, &adapter, 0.0).await.unwrap();
        assert_eq!(sent, 2);
        // 验证回放确实把帧发到了总线
        let f1 = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(f1.id.raw(), 0x7E0);
        let _ = std::fs::remove_file(&p);
    }
}
