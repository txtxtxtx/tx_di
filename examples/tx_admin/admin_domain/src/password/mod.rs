//! 密码安全哈希模块
//!
//! 使用 Argon2id 算法实现密码的安全存储和验证。
//!
//! # 安全特性
//! - **单向哈希**: 无法从哈希值反推原始密码
//! - **随机盐**: 每个密码使用独立的 16 字节随机盐
//! - **慢哈希**: Argon2id 设计为计算密集型，抵抗暴力破解
//!
//! # 存储格式
//! 哈希后的密码格式: `$argon2id$v=19$m=19456,t=2,p=1$<base64_salt>$<base64_hash>`

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2, Params, Version,
};
use tx_error::AppResult;

/// Argon2 参数配置
///
/// 根据 OWASP 推荐配置:
/// - 内存成本: 19456 KiB (19 MiB)
/// - 时间成本: 2 次迭代
/// - 并行度: 1 个线程
const MEMORY_COST: u32 = 19_456;
const TIME_COST: u32 = 2;
const PARALLELISM: u32 = 1;

/// 生成密码哈希
///
/// # 参数
/// - `password`: 原始密码（明文）
///
/// # 返回
/// - `Ok(String)`: 哈希后的密码字符串（包含算法参数和盐）
/// - `Err`: 哈希失败（通常不会发生）
///
/// # 示例
/// ```rust
/// use admin_domain::password::hash_password;
///
/// let hashed = hash_password("my_secure_password").unwrap();
/// assert!(hashed.starts_with("$argon2id$"));
/// ```
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);

    let params = Params::new(MEMORY_COST, TIME_COST, PARALLELISM, None)
        .map_err(|e| anyhow::anyhow!("Failed to create Argon2 params: {}", e))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    Ok(password_hash)
}

/// 验证密码是否匹配
///
/// # 参数
/// - `password`: 用户输入的原始密码（明文）
/// - `hashed_password`: 数据库中存储的哈希密码
///
/// # 返回
/// - `Ok(true)`: 密码匹配
/// - `Ok(false)`: 密码不匹配
/// - `Err`: 哈希格式无效或验证过程出错
///
/// # 示例
/// ```rust
/// use admin_domain::password::{hash_password, verify_password};
///
/// let hashed = hash_password("my_secure_password").unwrap();
/// assert!(verify_password("my_secure_password", &hashed).unwrap());
/// assert!(!verify_password("wrong_password", &hashed).unwrap());
/// ```
pub fn verify_password(password: &str, hashed_password: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(hashed_password)
        .map_err(|e| anyhow::anyhow!("Invalid password hash format: {}", e))?;

    let is_valid = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_format() {
        let password = "test_password_123";
        let hashed = hash_password(password).unwrap();

        // 验证格式: 应该以 $argon2id$ 开头
        assert!(hashed.starts_with("$argon2id$"), "哈希应该使用 argon2id 算法");

        // 验证格式: 应该包含版本号和参数
        assert!(hashed.contains("$v=19$"), "应该包含版本号 v=19");
        assert!(hashed.contains("m=19456"), "应该包含内存成本参数");
        assert!(hashed.contains("t=2"), "应该包含时间成本参数");
        assert!(hashed.contains("p=1"), "应该包含并行度参数");
    }

    #[test]
    fn test_hash_password_unique_salt() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // 相同密码应该产生不同的哈希（因为盐不同）
        assert_ne!(hash1, hash2, "相同密码的哈希应该不同（盐不同）");
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "correct_password";
        let hashed = hash_password(password).unwrap();

        assert!(
            verify_password(password, &hashed).unwrap(),
            "正确的密码应该验证通过"
        );
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hashed = hash_password(password).unwrap();

        assert!(
            !verify_password(wrong_password, &hashed).unwrap(),
            "错误的密码应该验证失败"
        );
    }

    #[test]
    fn test_verify_password_empty() {
        let password = "some_password";
        let hashed = hash_password(password).unwrap();

        assert!(
            !verify_password("", &hashed).unwrap(),
            "空密码应该验证失败"
        );
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("password", "invalid_hash_format");
        assert!(result.is_err(), "无效的哈希格式应该返回错误");
    }

    #[test]
    fn test_hash_password_unicode() {
        let password = "密码测试123!@#";
        let hashed = hash_password(password).unwrap();

        assert!(
            verify_password(password, &hashed).unwrap(),
            "Unicode 密码应该正确处理"
        );
    }

    #[test]
    fn test_hash_password_long() {
        // 测试长密码（超过 Argon2 的限制）
        let password = "a".repeat(1000);
        let hashed = hash_password(&password).unwrap();

        assert!(
            verify_password(&password, &hashed).unwrap(),
            "长密码应该正确处理"
        );
    }

    #[test]
    fn test_hash_password_special_chars() {
        let password = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let hashed = hash_password(password).unwrap();

        assert!(
            verify_password(password, &hashed).unwrap(),
            "特殊字符密码应该正确处理"
        );
    }

    #[test]
    fn test_performance_acceptable() {
        use std::time::Instant;

        let password = "performance_test_password";
        let start = Instant::now();

        // 哈希操作应该在合理时间内完成（通常 100-500ms）
        let _hashed = hash_password(password).unwrap();

        let duration = start.elapsed();
        assert!(
            duration.as_secs() < 5,
            "密码哈希应该在 5 秒内完成，实际耗时: {:?}",
            duration
        );
    }

    #[test]
    fn test_verify_performance() {
        let password = "verify_test_password";
        let hashed = hash_password(password).unwrap();

        let start = std::time::Instant::now();

        // 验证操作应该在合理时间内完成
        let result = verify_password(password, &hashed).unwrap();

        let duration = start.elapsed();
        assert!(result, "验证应该成功");
        assert!(
            duration.as_secs() < 5,
            "密码验证应该在 5 秒内完成，实际耗时: {:?}",
            duration
        );
    }
}
