//! GB28181 安全工具模块
//!
//! 从 `handlers.rs` 提取的密码学工具函数：
//! - 安全随机 Nonce 生成（替代原来的时间戳方案）
//! - MD5 摘要计算（GB28181 SIP 摘要认证专用）
//! - SIP Digest Auth 验证（RFC 2617）

// ── SIP Digest Auth 验证 ──────────────────────────────────────────────────
pub use tx_gb28181::utils::{generate_nonce, md5_digest, md5_hex, verify_digest_auth};


// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_is_hex_32() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32);
        assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn nonce_uniqueness() {
        let a = generate_nonce();
        let b = generate_nonce();
        assert_ne!(a, b);
    }

    #[test]
    fn md5_known_vector() {
        // RFC 1321 test vector: MD5("") = d41d8cd98f00b204e9800998ecf8427e
        let hash = md5_digest(b"");
        assert_eq!(md5_hex(hash), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_abc() {
        let hash = md5_digest(b"abc");
        assert_eq!(md5_hex(hash), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn digest_auth_valid() {
        let auth = "Digest username=\"34020000001320000001\", realm=\"3402000000\", nonce=\"testnonce\", response=\"correct\", uri=\"sip:34020000002000000001@192.168.1.100:5060\"";

        // 计算正确的 response
        let a1 = "34020000001320000001:3402000000:12345678";
        let a2 = "REGISTER:sip:34020000002000000001@192.168.1.100:5060";
        let ha1 = md5_hex(md5_digest(a1.as_bytes()));
        let ha2 = md5_hex(md5_digest(a2.as_bytes()));
        let correct_response = md5_hex(md5_digest(
            format!("{}:{}:{}", ha1, "testnonce", ha2).as_bytes(),
        ));

        let auth_with_correct = format!(
            "Digest username=\"34020000001320000001\", realm=\"3402000000\", \
             nonce=\"testnonce\", response=\"{}\"",
            correct_response
        );

        assert!(verify_digest_auth(
            &auth_with_correct,
            "REGISTER",
            "sip:34020000002000000001@192.168.1.100:5060",
            "12345678",
            "3402000000",
            "testnonce",
        ));
    }

    #[test]
    fn digest_auth_wrong_password() {
        let a1 = "user:realm:wrongpass";
        let a2 = "REGISTER:sip:target@1.2.3.4:5060";
        let ha1 = md5_hex(md5_digest(a1.as_bytes()));
        let ha2 = md5_hex(md5_digest(a2.as_bytes()));
        let response = md5_hex(md5_digest(
            format!("{}:nonce:{}", ha1, ha2).as_bytes(),
        ));

        let auth = format!(
            "Digest username=\"user\", realm=\"realm\", nonce=\"nonce\", response=\"{}\"",
            response
        );

        // 使用不同的密码验证
        assert!(!verify_digest_auth(
            &auth,
            "REGISTER",
            "sip:target@1.2.3.4:5060",
            "correct_password", // 不匹配
            "realm",
            "nonce",
        ));
    }
}
