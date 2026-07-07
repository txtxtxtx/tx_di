//! in-dialog 关联助手
//!
//! SIP 对话（dialog）由三元组唯一标识（RFC 3261 §12）：
//!
//! ```text
//! Dialog-ID = Call-ID + From-Tag + To-Tag
//! ```
//!
//! 初始请求（如 INVITE）的 `To` 头尚未携带 to-tag（由 UAS 在响应中填入），
//! 因此请求本身只能形成「半对话键」；而后续 in-dialog 请求（ACK / BYE / INFO /
//! MESSAGE）的 `To` 头已含 to-tag，可用完整三元组精确关联回原始对话。
//!
//! 本模块提供：
//! - [`DialogKey`]：对话关联键（含构造与解析）；
//! - [`InDialogTable<T>`]：按 `DialogKey` 关联任意对话上下文（如媒体会话），
//!   供上层在收到 in-dialog 请求时快速查表。

use dashmap::DashMap;
use rsipstack::sip::HeadersExt;
use rsipstack::sip::Request;
use std::sync::Arc;

/// 对话关联键：Call-ID + From-Tag + To-Tag
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DialogKey {
    /// SIP Call-ID
    pub call_id: String,
    /// From 头中的 tag（发起方）
    pub from_tag: String,
    /// To 头中的 tag（被叫方；初始请求可能为 None）
    pub to_tag: Option<String>,
}

impl DialogKey {
    /// 从请求头提取对话键。
    ///
    /// - `call_id` 来自 `Call-ID` 头；
    /// - `from_tag` 来自 `From` 头的 `tag` 参数（缺失则返回 `None`）；
    /// - `to_tag` 来自 `To` 头的 `tag` 参数（初始请求尚未填充，故为 `Option`）。
    ///
    /// 返回 `None` 表示 Call-ID 或 From-Tag 缺失（非法对话）。
    pub fn from_request(req: &Request) -> Option<DialogKey> {
        // 用 typed header 的 value() 取原始头值字符串（含 tag 参数）
        let call_id = req.call_id_header().ok()?.value().to_string();
        let from = req.from_header().ok()?.value().to_string();
        let to = req
            .to_header()
            .ok()
            .map(|h| h.value().to_string())
            .unwrap_or_default();
        let from_tag = extract_tag(&from)?;
        let to_tag = extract_tag(&to);
        Some(DialogKey {
            call_id,
            from_tag,
            to_tag,
        })
    }

    /// 生成「完整」对话键：要求 to_tag 已确定。
    ///
    /// 用于把「半对话键」（to_tag = None）与响应中分配的 to_tag 合并，
    /// 得到可入表的完整键。
    pub fn with_to_tag(&self, to_tag: impl Into<String>) -> DialogKey {
        DialogKey {
            call_id: self.call_id.clone(),
            from_tag: self.from_tag.clone(),
            to_tag: Some(to_tag.into()),
        }
    }
}

/// 从 `From` / `To` 头的值字符串中提取 `tag` 参数。
///
/// 例如 `<sip:alice@a.com>;tag=abc123` → `Some("abc123")`。
fn extract_tag(header_value: &str) -> Option<String> {
    let idx = header_value.find(";tag=")?;
    let rest = &header_value[idx + 5..];
    // tag 之后可能还有其它参数（;...），取到分号或行尾为止
    let tag = rest.split(';').next().unwrap_or("").trim();
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

/// in-dialog 关联表：按 [`DialogKey`] 关联对话上下文 `T`。
///
/// 适用于需要在收到 in-dialog 请求（BYE / INFO / MESSAGE）时，
/// 快速定位原始对话（如媒体会话、设备通道）的场景。
#[derive(Default)]
pub struct InDialogTable<T: Clone + Send + Sync + 'static> {
    map: DashMap<DialogKey, Arc<T>>,
}

impl<T: Clone + Send + Sync + 'static> InDialogTable<T> {
    /// 新建空表
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// 插入 / 覆盖一条对话上下文
    pub fn insert(&self, key: DialogKey, value: T) {
        self.map.insert(key, Arc::new(value));
    }

    /// 按完整对话键查询上下文（in-dialog 请求使用）
    pub fn lookup(&self, key: &DialogKey) -> Option<Arc<T>> {
        // 优先完整匹配（to_tag 已知）
        if let Some(v) = self.map.get(key) {
            return Some(v.clone());
        }
        // 回退：to_tag 未知时，匹配 call_id+from_tag 且表中 to_tag 为 Some 的条目
        if key.to_tag.is_none() {
            for entry in self.map.iter() {
                if entry.key().call_id == key.call_id
                    && entry.key().from_tag == key.from_tag
                    && entry.key().to_tag.is_some()
                {
                    return Some(entry.value().clone());
                }
            }
        }
        None
    }

    /// 移除一条对话上下文（如收到 BYE 后清理）
    pub fn remove(&self, key: &DialogKey) {
        self.map.remove(key);
        // 同时尝试按 call_id+from_tag 清理（to_tag 可能未对齐）
        if key.to_tag.is_none() {
            self.map.retain(|k, _| {
                !(k.call_id == key.call_id
                    && k.from_tag == key.from_tag
                    && k.to_tag.is_some())
            });
        }
    }

    /// 当前关联数量
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsipstack::sip::{CallId, From, Header, Request, To, Uri};

    fn make_request(call_id: &str, from_tag: &str, to_tag: Option<&str>) -> Request {
        let from_val = format!("<sip:caller@a.com>;tag={}", from_tag);
        let to_val = match to_tag {
            Some(t) => format!("<sip:callee@b.com>;tag={}", t),
            None => "<sip:callee@b.com>".to_string(),
        };
        Request {
            method: rsipstack::sip::Method::Invite,
            uri: Uri::try_from("sip:callee@b.com").unwrap(),
            headers: vec![
                Header::CallId(CallId::new(call_id.to_string())),
                Header::From(From::new(from_val)),
                Header::To(To::new(to_val)),
            ]
            .into(),
            body: vec![],
            version: rsipstack::sip::Version::V2,
        }
    }

    #[test]
    fn extract_tag_present() {
        assert_eq!(
            extract_tag("<sip:a@b.com>;tag=XYZ"),
            Some("XYZ".to_string())
        );
    }

    #[test]
    fn extract_tag_absent() {
        assert_eq!(extract_tag("<sip:a@b.com>"), None);
    }

    #[test]
    fn extract_tag_with_extra_params() {
        assert_eq!(
            extract_tag("<sip:a@b.com>;tag=ABC;other=1"),
            Some("ABC".to_string())
        );
    }

    #[test]
    fn dialog_key_from_request_initial_no_to_tag() {
        let req = make_request("C1", "F1", None);
        let key = DialogKey::from_request(&req).expect("应提取到键");
        assert_eq!(key.call_id, "C1");
        assert_eq!(key.from_tag, "F1");
        assert_eq!(key.to_tag, None);
    }

    #[test]
    fn dialog_key_from_request_with_to_tag() {
        let req = make_request("C1", "F1", Some("T1"));
        let key = DialogKey::from_request(&req).expect("应提取到键");
        assert_eq!(key.to_tag, Some("T1".to_string()));
    }

    #[test]
    fn dialog_key_from_request_missing_call_id_returns_none() {
        let mut req = make_request("C1", "F1", None);
        // 移除 Call-ID 头
        let headers: Vec<Header> = req
            .headers
            .iter()
            .filter(|h| !matches!(h, Header::CallId(_)))
            .cloned()
            .collect();
        req.headers = headers.into();
        assert!(DialogKey::from_request(&req).is_none());
    }

    #[test]
    fn in_dialog_table_insert_lookup_remove() {
        let table: InDialogTable<u32> = InDialogTable::new();
        let half = DialogKey {
            call_id: "C1".into(),
            from_tag: "F1".into(),
            to_tag: None,
        };
        // 初始请求阶段只能拿到半键，无法精确入表；
        // 这里模拟响应分配 to_tag 后插入完整键
        let full = half.with_to_tag("T1");
        table.insert(full.clone(), 42);
        assert_eq!(table.len(), 1);

        // in-dialog 请求携带完整键 → 精确命中
        let req = make_request("C1", "F1", Some("T1"));
        let key = DialogKey::from_request(&req).unwrap();
        assert_eq!(table.lookup(&key), Some(Arc::new(42)));

        // 回退：in-dialog 请求若只传半键，也能关联到已建立的对话
        assert_eq!(table.lookup(&half), Some(Arc::new(42)));

        table.remove(&key);
        assert!(table.is_empty());
    }
}
