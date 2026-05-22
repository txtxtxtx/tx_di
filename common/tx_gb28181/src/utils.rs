use rand::Rng;

/// 生成加密安全的随机 Nonce（16 字节 → 32 位十六进制字符串）
///
/// 使用 `rand` crate 的密码学安全随机数生成器，替代原来的时间戳+黄金比例散列方案。
/// 32 位十六进制字符 = 128 bit 随机数，抗碰撞且不可预测。
pub fn generate_nonce() -> String {
    let bytes: [u8; 16] = rand::rng().random();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];
/// ── MD5 实现（RFC 1321）
///
/// GB28181 SIP 摘要认证仅使用 MD5，无需引入额外依赖。
///
/// 计算数据的 MD5 摘要
pub fn md5_digest(data: &[u8]) -> [u8; 16] {
    let mut state = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];

    // 填充
    let orig_len_bits = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_le_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 16];
        for (i, b) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        for i in 0..64usize {
            let (f, g): (u32, usize) = if i < 16 {
                ((b & c) | (!b & d), i)
            } else if i < 32 {
                ((d & b) | (!d & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | !d), (7 * i) % 16)
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(w[g])
                    .rotate_left(S[i]),
            );
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    let mut result = [0u8; 16];
    for (i, &s) in state.iter().enumerate() {
        let b = s.to_le_bytes();
        result[i * 4..i * 4 + 4].copy_from_slice(&b);
    }
    result
}

/// 将 MD5 结果转换为十六进制字符串
pub fn md5_hex(hash: [u8; 16]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 验证 SIP 摘要认证
///
/// 按 RFC 2617 / GB28181 规范验证 Authorization 头：
/// `response = MD5(MD5(A1):nonce:MD5(A2))`
/// 其中 A1 = `username:realm:password`，A2 = `method:uri`
pub fn verify_digest_auth(
    auth_header: &str,   // Authorization 头内容
    method: &str,  // HTTP/SIP 方法（如 REGISTER）
    uri: &str,  // 请求 URI
    expected_password: &str, // 期望的密码
    realm: &str,  // 认证域
    nonce: &str, // 服务器生成的随机数
) -> bool {
    // 从 Authorization 头提取各字段
    let get_field = |name: &str| -> Option<String> {
        let prefix = format!("{}=\"", name);
        let start = auth_header.find(&prefix)? + prefix.len();
        let end = auth_header[start..].find('"')?;
        Some(auth_header[start..start + end].to_string())
    };

    let auth_response = match get_field("response") {
        Some(r) => r,
        None => return false,
    };
    let auth_nonce = get_field("nonce").unwrap_or_default();
    let auth_realm = get_field("realm").unwrap_or_default();
    let auth_username = get_field("username").unwrap_or_default();

    // 验证 realm 匹配
    if auth_realm != realm {
        tracing::debug!("realm 不匹配: expected={}, got={}", realm, auth_realm);
        return false;
    }

    // 验证 nonce 匹配
    if auth_nonce != nonce {
        tracing::debug!("nonce 不匹配: expected={}, got={}", nonce, auth_nonce);
        return false;
    }

    // 计算期望的 response
    let a1 = format!("{}:{}:{}", auth_username, realm, expected_password);
    let a2 = format!("{}:{}", method, uri);
    let ha1 = md5_hex(md5_digest(a1.as_bytes()));
    let ha2 = md5_hex(md5_digest(a2.as_bytes()));
    let expected = md5_hex(md5_digest(format!("{}:{}:{}", ha1, nonce, ha2).as_bytes()));

    if auth_response == expected {
        true
    } else {
        tracing::debug!(
            "摘要验证失败: username={}, realm={}, uri={}",
            auth_username, realm, uri
        );
        false
    }
}