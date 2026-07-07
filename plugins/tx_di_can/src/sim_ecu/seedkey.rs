//! Seed-Key 安全访问算法
//!
//! ECU 仿真节点（应答方）与 UDS 客户端（请求方 `key_fn`）共用同一组算法，
//! 保证无设备联调时 seed→key 一致。

/// 可选算法（GUI 后续可切换；按 security_level 默认映射）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedKeyAlgo {
    /// 简单异或：key[i] = seed[i] ^ 0xA5
    Xor,
    /// 乘加位移：key[i] = (seed[i] * (7+level) + level) << 1 ^ 0xA5
    Mul,
    /// 取反：key[i] = !seed[i]
    Not,
}

impl SeedKeyAlgo {
    /// 由 security level（奇数，请求 seed）映射到默认算法
    pub fn for_level(level: u8) -> SeedKeyAlgo {
        match level {
            0x01 => SeedKeyAlgo::Xor,
            0x03 => SeedKeyAlgo::Mul,
            0x05 => SeedKeyAlgo::Not,
            _ => SeedKeyAlgo::Xor,
        }
    }
}

/// 计算 key：算法由 `level` 决定，与 ECU 端一致
pub fn compute_key(seed: &[u8], level: u8) -> Vec<u8> {
    let algo = SeedKeyAlgo::for_level(level);
    match algo {
        SeedKeyAlgo::Xor => seed.iter().map(|&s| s ^ 0xA5).collect(),
        SeedKeyAlgo::Mul => seed
            .iter()
            .map(|&s| (s.wrapping_mul(7u8.wrapping_add(level))).wrapping_add(level) << 1 ^ 0xA5)
            .collect(),
        SeedKeyAlgo::Not => seed.iter().map(|&s| !s).collect(),
    }
}

/// 生成随机种子（仿真用，固定可复现长度的种子）
pub fn generate_seed(len: usize, level: u8) -> Vec<u8> {
    // 使用确定性序列便于测试；真实 ECU 应使用真随机
    (0..len).map(|i| ((i as u8).wrapping_mul(0x11)).wrapping_add(level)).collect()
}
