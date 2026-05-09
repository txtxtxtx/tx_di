//! SIP 工具函数
//!
//! 轻量级 SIP URI 解析工具，供服务端插件、客户端插件、示例程序共用。
//! 不依赖 SIP 协议栈（rsipstack），仅做字符串级别的解析。

/// 从 SIP URI 字符串中提取 user 部分（即设备编号）
///
/// 支持以下格式：
/// - `<sip:34020000001320000001@192.168.1.200>` → `Some("34020000001320000001")`
/// - `<sip:34020000001320000001@192.168.1.200>;expires=3600` → `Some("34020000001320000001")`
/// - `sip:34020000001320000001@192.168.1.200` → `Some("34020000001320000001")`
/// - `sips:user@host` → `Some("user")`
///
/// # 示例
///
/// ```
/// use tx_gb28181::sip::extract_user_from_sip_uri;
///
/// let uri = "<sip:34020000001320000001@192.168.1.200:5060>";
/// assert_eq!(extract_user_from_sip_uri(uri), Some("34020000001320000001".into()));
///
/// let uri2 = "<sip:34020000001320000001@192.168.1.200>;expires=3600";
/// assert_eq!(extract_user_from_sip_uri(uri2), Some("34020000001320000001".into()));
/// ```
pub fn extract_user_from_sip_uri(uri_str: &str) -> Option<String> {
    // 去掉尖括号及 display-name
    let clean = uri_str
        .trim()
        .trim_start_matches('"')
        .trim();

    // 提取 < > 内的部分
    let inner = if let (Some(s), Some(e)) = (clean.find('<'), clean.rfind('>')) {
        &clean[s + 1..e]
    } else {
        clean
    };

    // 去掉 sip: 前缀
    let after_scheme = inner
        .strip_prefix("sip:")
        .or_else(|| inner.strip_prefix("sips:"))
        .unwrap_or(inner);

    // 取 @ 之前的 user 部分（去掉 ;tag=xxx 等参数）
    let user_part = after_scheme.split('@').next().unwrap_or(after_scheme);
    // 去掉可能的参数
    let user = user_part.split(';').next().unwrap_or(user_part);

    if user.is_empty() {
        None
    } else {
        Some(user.to_string())
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_angle_brackets() {
        let uri = "<sip:34020000001320000001@192.168.1.200>";
        assert_eq!(
            extract_user_from_sip_uri(uri),
            Some("34020000001320000001".into())
        );
    }

    #[test]
    fn extract_with_expires_param() {
        let uri = "<sip:34020000001320000001@192.168.1.200:5060>;expires=3600";
        assert_eq!(
            extract_user_from_sip_uri(uri),
            Some("34020000001320000001".into())
        );
    }

    #[test]
    fn extract_without_angle_brackets() {
        let uri = "sip:34020000001320000001@192.168.1.200";
        assert_eq!(
            extract_user_from_sip_uri(uri),
            Some("34020000001320000001".into())
        );
    }

    #[test]
    fn extract_sips() {
        let uri = "sips:user@host";
        assert_eq!(
            extract_user_from_sip_uri(uri),
            Some("user".into())
        );
    }

    #[test]
    fn extract_with_tag() {
        let uri = "<sip:user@host>;tag=abc123";
        assert_eq!(
            extract_user_from_sip_uri(uri),
            Some("user".into())
        );
    }

    #[test]
    fn extract_empty_returns_none() {
        let uri = "<sip:@host>";
        assert_eq!(extract_user_from_sip_uri(uri), None);
    }

    #[test]
    fn extract_with_display_name() {
        let uri = r#""Device 001" <sip:34020000001320000001@192.168.1.200>"#;
        assert_eq!(
            extract_user_from_sip_uri(uri),
            Some("34020000001320000001".into())
        );
    }
}
