//! 操作审计日志（内存环形缓冲）
//!
//! 记录上位机关键操作（连接、收发、UDS、刷写、录制/回放、XCP 等）的时间、类型、
//! 详情与结果，供审计报表导出（HTML/PDF）与产线留痕。

use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// 单条审计记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    /// UNIX 毫秒时间戳
    pub ts_ms: u64,
    /// 操作类型：connect / send / read_did / write_did / flash / record / replay / isotp / xcp
    pub kind: String,
    /// 详情（如目标 ID、DID、文件路径等）
    pub detail: String,
    /// 结果：ok / fail:<msg>
    pub result: String,
}

static AUDIT: OnceLock<Mutex<Vec<AuditEntry>>> = OnceLock::new();

fn store() -> &'static Mutex<Vec<AuditEntry>> {
    AUDIT.get_or_init(|| Mutex::new(Vec::new()))
}

/// 记录一条审计（自动打时间戳）
pub fn record(kind: &str, detail: &str, result: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let entry = AuditEntry {
        ts_ms: ts,
        kind: kind.to_string(),
        detail: detail.to_string(),
        result: result.to_string(),
    };
    if let Ok(mut g) = store().lock() {
        g.push(entry);
        // 限制内存占用：最多保留 10000 条
        if g.len() > 10000 {
            let drop = g.len() - 10000;
            g.drain(0..drop);
        }
    }
}

/// 记录成功
pub fn ok(kind: &str, detail: &str) {
    record(kind, detail, "ok");
}

/// 记录失败
pub fn fail(kind: &str, detail: &str, err: &str) {
    record(kind, detail, &format!("fail:{}", err));
}

/// 读取全部审计记录（克隆）
pub fn log() -> Vec<AuditEntry> {
    store().lock().map(|g| g.clone()).unwrap_or_default()
}

/// 清空审计日志
pub fn clear() {
    if let Ok(mut g) = store().lock() {
        g.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_record_and_read() {
        clear();
        ok("connect", "simbus");
        fail("flash", "fw.bin", "timeout");
        let entries = log();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, "connect");
        assert_eq!(entries[0].result, "ok");
        assert!(entries[1].result.starts_with("fail:"));
    }
}
