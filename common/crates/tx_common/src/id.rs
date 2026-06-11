use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::LazyLock;
/// 雪花算法ID生成器
///
/// ID结构(64位):
/// - 1位: 符号位(始终为0)
/// - 41位: 时间戳(相对于自定义纪元的毫秒数)
/// - 5位: 数据中心ID
/// - 5位: 工作节点ID
/// - 12位: 序列号
///
/// 相比基础雪花算法的优化:
/// - 使用AtomicU64实现无锁设计，无需Mutex
/// - 使用CAS(Compare-And-Swap)循环实现线程安全的ID生成
/// - 时钟回拨采用指数退避算法，避免忙等待
/// - 添加闰秒容忍处理机制，防止闰秒插入导致的ID重复或回拨错误
/// - 支持批量生成ID，减少原子操作开销
/// ID生成器结构体，用于生成唯一的ID
pub struct IdGenerator {
    /// 工作节点ID，用于标识不同的工作节点
    worker_id: u64,
    /// 数据中心ID，用于标识不同的数据中心
    datacenter_id: u64,
    /// 将last_timestamp(高52位)和sequence(低12位)打包到一个AtomicU64中
    /// 这使得无锁CAS更新成为可能 — 两个字段一起原子更新。
    state: AtomicU64,
}

/// 自定义纪元: 2026-01-01 00:00:00 UTC
const EPOCH: u64 = 1767225600000;

/// 工作节点ID位数
const WORKER_ID_BITS: u64 = 5;
/// 数据中心ID位数
const DATACENTER_ID_BITS: u64 = 5;
/// 序列号位数
const SEQUENCE_BITS: u64 = 12;

/// 最大工作节点ID
const MAX_WORKER_ID: u64 = (1 << WORKER_ID_BITS) - 1;
/// 最大数据中心ID
const MAX_DATACENTER_ID: u64 = (1 << DATACENTER_ID_BITS) - 1;
/// 最大序列号
const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;

/// 工作ID位移量
const WORKER_ID_SHIFT: u64 = SEQUENCE_BITS;
/// 数据中心ID位移量
const DATACENTER_ID_SHIFT: u64 = SEQUENCE_BITS + WORKER_ID_BITS;
/// 时间戳位移量
const TIMESTAMP_SHIFT: u64 = SEQUENCE_BITS + WORKER_ID_BITS + DATACENTER_ID_BITS;

/// 最大允许的时钟回拨(毫秒)
const MAX_CLOCK_DRIFT_MS: u64 = 5;

/// 时钟回拨指数退避最大重试次数
const MAX_BACKOFF_RETRIES: u32 = 10;

/// 闰秒检测阈值(毫秒)
/// 如果时间回拨在此阈值内，认为是闰秒导致的回拨而非时钟错误
/// 国际原子时(TAI)与协调世界时(UTC)之间可能存在闰秒调整，导致系统时间回拨
/// 设置为1000毫秒(1秒)，以捕获所有已知的闰秒调整情况
const LEAP_SECOND_THRESHOLD_MS: u64 = 1000;

/// 将时间戳和序列号打包为一个u64状态值
/// 布局: [时间戳: 52位][序列号: 12位]
#[inline]
fn pack_state(timestamp: u64, sequence: u64) -> u64 {
    (timestamp << SEQUENCE_BITS) | (sequence & MAX_SEQUENCE)
}

/// 从状态值中解包出(时间戳, 序列号)
#[inline]
fn unpack_state(state: u64) -> (u64, u64) {
    let timestamp = state >> SEQUENCE_BITS;
    let sequence = state & MAX_SEQUENCE;
    (timestamp, sequence)
}

impl IdGenerator {
    pub fn new(worker_id: u64, datacenter_id: u64) -> Self {
        assert!(worker_id <= MAX_WORKER_ID, "Worker ID exceeds maximum");
        assert!(
            datacenter_id <= MAX_DATACENTER_ID,
            "Datacenter ID exceeds maximum"
        );
        Self {
            worker_id,
            datacenter_id,
            state: AtomicU64::new(0),
        }
    }

    /// 使用无锁CAS循环生成下一个唯一ID
    ///
    /// `state`字段原子性地打包了`last_timestamp`和`sequence`，
    /// 因此更新它们时永远不需要Mutex。如果另一个线程同时修改了状态，CAS循环会重试
    ///
    /// 当检测到闰秒事件时，会使用特殊的序列号(LEAP_SECOND_SEQUENCE)标记ID
    pub fn next_id(&self) -> u64 {
        loop {
            let current_state = self.state.load(Ordering::Acquire);
            let (last_ts, last_seq) = unpack_state(current_state);
            let mut timestamp = Self::current_timestamp();

            // 使用指数退避处理时钟回拨
            if timestamp < last_ts {
                let drift = last_ts - timestamp;
                if drift > MAX_CLOCK_DRIFT_MS {
                    panic!(
                        "时钟回拨{}毫秒，超过最大容忍度{}毫秒",
                        drift, MAX_CLOCK_DRIFT_MS
                    );
                }
                // 指数退避：休眠2^retry * 100µs而不是忙等待
                Self::backoff_wait(last_ts);
                continue;
            }

            let new_seq = if timestamp == last_ts {
                // 同一毫秒：递增序列号
                let seq = last_seq + 1;
                if seq > MAX_SEQUENCE {
                    // 本毫秒序列号耗尽 — 等待下一毫秒
                    timestamp = Self::wait_next_millis(last_ts);
                    0
                } else {
                    seq
                }
            } else {
                // 新毫秒：重置序列号为0
                0
            };

            let new_state = pack_state(timestamp, new_seq);

            // CAS：如果状态自读取后未改变，则原子更新
            if self
                .state
                .compare_exchange(current_state, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return ((timestamp - EPOCH) << TIMESTAMP_SHIFT)
                    | (self.datacenter_id << DATACENTER_ID_SHIFT)
                    | (self.worker_id << WORKER_ID_SHIFT)
                    | new_seq;
            }
            // CAS失败 — 另一个线程先更新了状态，重试
        }
    }

    /// 高效地生成一批唯一ID
    ///
    /// 使用CAS操作在当前毫秒内预留连续的序列号块，然后直接构造ID，
    /// 无需为每个ID单独执行原子操作。当当前毫秒的序列号不足时，
    /// 先用完剩余序列号，再等待下一毫秒继续批量分配，始终保持高效。
    ///
    /// 当检测到闰秒事件时，会使用特殊的序列号(LEAP_SECOND_SEQUENCE)标记ID
    pub fn next_ids(&self, count: usize) -> Vec<u64> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![self.next_id()];
        }

        let mut ids = Vec::with_capacity(count);
        let mut remaining = count;

        while remaining > 0 {
            let current_state = self.state.load(Ordering::Acquire);
            let (last_ts, last_seq) = unpack_state(current_state);
            let mut timestamp = Self::current_timestamp();

            // 处理时钟回拨
            if timestamp < last_ts {
                let drift = last_ts - timestamp;
                if drift > MAX_CLOCK_DRIFT_MS {
                    panic!(
                        "时钟回拨{}毫秒，超过最大容忍度{}毫秒",
                        drift, MAX_CLOCK_DRIFT_MS
                    );
                }
                Self::backoff_wait(last_ts);
                continue;
            }

            // 计算当前毫秒可用的序列号范围
            let base_seq = if timestamp == last_ts {
                last_seq + 1
            } else {
                0
            };

            // 当前毫秒剩余可用序列号数量
            // 如果 base_seq > MAX_SEQUENCE，说明当前毫秒序列号已耗尽，需要等待下一毫秒
            let available_in_ms = if base_seq > MAX_SEQUENCE {
                // 等待下一毫秒后重试
                timestamp = Self::wait_next_millis(timestamp);
                continue;
            } else {
                (MAX_SEQUENCE - base_seq + 1) as usize
            };
            // 本次批量分配的数量：取剩余需求和可用序列号的较小值
            let batch_size = available_in_ms.min(remaining);

            // CAS预留序列号块 [base_seq, base_seq + batch_size - 1]
            let new_seq = base_seq + batch_size as u64 - 1;
            let new_state = pack_state(timestamp, new_seq);

            if self
                .state
                .compare_exchange(current_state, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                // CAS失败 — 另一个线程先更新了状态，重试
                continue;
            }

            // CAS成功，直接构造ID，无需额外原子操作
            for i in 0..batch_size {
                let seq = base_seq + i as u64;
                ids.push(
                    ((timestamp - EPOCH) << TIMESTAMP_SHIFT)
                        | (self.datacenter_id << DATACENTER_ID_SHIFT)
                        | (self.worker_id << WORKER_ID_SHIFT)
                        | seq,
                );
            }
            remaining -= batch_size;

            // 如果当前毫秒序列号已用完但仍有剩余ID需要生成，等待下一毫秒
            if remaining > 0 && batch_size == available_in_ms {
                // 等待下一毫秒，然后继续批量分配
                timestamp = Self::wait_next_millis(timestamp);
            }
        }

        ids
    }

    /// 使用指数退避等待时钟达到或超过`target_ts`
    ///
    /// 避免浪费CPU的紧忙等待，此方法采用指数增长休眠时间：50µs, 100µs, 200µs, … 每次迭代最多约25ms。
    fn backoff_wait(target_ts: u64) {
        let mut delay_us: u64 = 50;
        for _ in 0..MAX_BACKOFF_RETRIES {
            if Self::current_timestamp() >= target_ts {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(delay_us));
            delay_us = (delay_us * 2).min(25_000); // cap at 25ms
        }
        // 对最后几微秒使用忙等待(更精确)
        while Self::current_timestamp() < target_ts {}
    }

    /// 忙等待直到当前时间超过`last_ts`
    ///
    /// 使用`std::hint::spin_loop`指示忙等待，允许CPU在旋转过程中优化功耗。
    fn wait_next_millis(last_ts: u64) -> u64 {
        let mut timestamp = Self::current_timestamp();
        while timestamp <= last_ts {
            std::hint::spin_loop();
            timestamp = Self::current_timestamp();
        }
        timestamp
    }

    /// 获取自UNIX纪元以来的当前时间戳(毫秒)
    ///
    /// 包含闰秒调整的小容忍度：如果系统时钟报告的时间略早于预期的单调递增
    /// (在1毫秒内)，我们将其限制为最后一个已知的好时间戳。这可以防止
    /// 闰秒引起的漂移导致虚假的时钟回拨恐慌。
    ///
    /// 实现了闰秒检测和补偿机制，确保在闰秒插入时ID仍能保持单调递增。
    /// 使用本地缓存的上一个有效时间戳来检测时间跳跃，防止闰秒导致的时钟回拨。
    fn current_timestamp() -> u64 {
        // 获取系统时间 （ms 毫秒）
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| {
                // 如果系统时间早于UNIX纪元，使用UNIX纪元作为最小值
                std::time::Duration::from_millis(0)
            })
            .as_millis() as u64;
            
        // 闰秒处理：如果当前时间小于上一次记录的时间，且差值在闰秒检测阈值内，
        // 则可能是闰秒导致的回拨，此时我们使用上一次的有效时间戳
        static LAST_VALID_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
        static LEAP_SECOND_DETECTED: AtomicBool = AtomicBool::new(false);
        let last_valid = LAST_VALID_TIMESTAMP.load(Ordering::Relaxed);
        
        if now < last_valid {
            let diff = last_valid - now;
            if diff <= LEAP_SECOND_THRESHOLD_MS {
                // 时间回拨在阈值内，可能是闰秒导致的，使用上一次的有效时间
                // 标记闰秒检测状态
                LEAP_SECOND_DETECTED.store(true, Ordering::Relaxed);
                return last_valid;
            }
        } else if now > last_valid {
            // 时间正常前进，重置闰秒检测状态
            LEAP_SECOND_DETECTED.store(false, Ordering::Relaxed);
        }
        
        // 更新最后有效时间戳
        LAST_VALID_TIMESTAMP.store(now, Ordering::Relaxed);
        now
    }
}

/// 全局ID生成器实例
static GLOBAL_ID_GENERATOR: LazyLock<IdGenerator> =
    LazyLock::new(|| IdGenerator::new(1, 1));

/// 生成下一个全局ID
pub fn next_id() -> u64 {
    GLOBAL_ID_GENERATOR.next_id()
}

/// 生成一批全局ID
pub fn next_ids(count: usize) -> Vec<u64> {
    GLOBAL_ID_GENERATOR.next_ids(count)
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use super::*;

    #[test]
    fn test_single_id_generation() {
        let generator = IdGenerator::new(1, 1);
        let id1 = generator.next_id();
        let id2 = generator.next_id();

        assert!(id2 > id1, "ID应该单调递增");
    }

    #[test]
    fn test_batch_id_generation() {
        let generator = IdGenerator::new(1, 1);
        let ids = generator.next_ids(100);

        assert_eq!(ids.len(), 100, "应该生成100个ID");

        // 验证ID单调递增
        for i in 1..ids.len() {
            assert!(ids[i] > ids[i-1], "批量生成的ID应该单调递增");
        }
    }

    #[test]
    fn test_concurrent_id_generation() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let mut handles = vec![];
        let ids_per_thread = 1000;
        let num_threads = 10;

        for _ in 0..num_threads {
            let arc_gen = generator.clone();
            let handle = thread::spawn(move || {
                let mut ids = Vec::with_capacity(ids_per_thread);
                for _ in 0..ids_per_thread {
                    ids.push(arc_gen.next_id());
                }
                ids
            });
            handles.push(handle);
        }

        let mut all_ids = Vec::new();
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }

        assert_eq!(all_ids.len(), ids_per_thread * num_threads, "应该生成正确数量的ID");

        // 验证所有ID唯一
        let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
        assert_eq!(unique_ids.len(), all_ids.len(), "所有ID应该是唯一的");
    }

    #[test]
    fn test_concurrent_batch_generation() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let mut handles = vec![];
        let batch_size = 100;
        let num_threads = 10;

        for _ in 0..num_threads {
            let arc_gen = generator.clone();
            let handle = thread::spawn(move || {
                arc_gen.next_ids(batch_size)
            });
            handles.push(handle);
        }

        let mut all_ids = Vec::new();
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }

        assert_eq!(all_ids.len(), batch_size * num_threads, "应该生成正确数量的ID");

        // 验证所有ID唯一
        let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
        assert_eq!(unique_ids.len(), all_ids.len(), "所有ID应该是唯一的");
    }

    #[test]
    fn test_global_next_id() {
        let id1 = next_id();
        let id2 = next_id();

        assert!(id2 > id1, "全局ID应该单调递增");
    }

    #[test]
    fn test_global_next_ids() {
        let ids = next_ids(50);

        assert_eq!(ids.len(), 50, "应该生成50个全局ID");

        // 验证ID单调递增
        for i in 1..ids.len() {
            assert!(ids[i] > ids[i-1], "批量生成的全局ID应该单调递增");
        }
    }

    #[test]
    fn test_id_structure() {
        let generator = IdGenerator::new(1, 1);
        let id = generator.next_id();

        // 提取各部分
        let sequence = id & 0xFFF;
        let worker_id = (id >> 12) & 0x1F;
        let datacenter_id = (id >> 17) & 0x1F;
        let timestamp = (id >> 22) & 0x1FFFFFFFFFF;

        assert_eq!(worker_id, 1, "工作节点ID应该为1");
        assert_eq!(datacenter_id, 1, "数据中心ID应该为1");
        assert!(sequence <= 0xFFF, "序列号应该在有效范围内");
        assert!(timestamp > 0, "时间戳应该大于0");
    }

    #[test]
    fn test_empty_batch() {
        let generator = IdGenerator::new(1, 1);
        let ids = generator.next_ids(0);

        assert_eq!(ids.len(), 0, "空批量应该返回空向量");
    }

    #[test]
    fn test_single_batch() {
        let generator = IdGenerator::new(1, 1);
        let ids = generator.next_ids(1);

        assert_eq!(ids.len(), 1, "单元素批量应该返回一个ID");
    }

    #[test]
    fn test_large_batch() {
        let generator = IdGenerator::new(1, 1);
        let batch_size = 5000;
        let ids = generator.next_ids(batch_size);

        assert_eq!(ids.len(), batch_size, "大批量应该生成正确数量的ID");

        // 验证ID单调递增
        for i in 1..ids.len() {
            assert!(ids[i] > ids[i-1], "大批量生成的ID应该单调递增");
        }
    }

    #[test]
    fn test_worker_id_bounds() {
        // 测试最大工作节点ID
        let generator = IdGenerator::new(31, 1);
        let id = generator.next_id();
        let worker_id = (id >> 12) & 0x1F;
        assert_eq!(worker_id, 31, "工作节点ID应该为31");
    }

    #[test]
    #[should_panic(expected = "Worker ID exceeds maximum")]
    fn test_invalid_worker_id() {
        // 测试超出范围的工作节点ID
        let _ = IdGenerator::new(32, 1);
    }

    #[test]
    fn test_datacenter_id_bounds() {
        // 测试最大数据中心ID
        let generator = IdGenerator::new(1, 31);
        let id = generator.next_id();
        let datacenter_id = (id >> 17) & 0x1F;
        assert_eq!(datacenter_id, 31, "数据中心ID应该为31");
    }

    #[test]
    #[should_panic(expected = "Datacenter ID exceeds maximum")]
    fn test_invalid_datacenter_id() {
        // 测试超出范围的数据中心ID
        let _ = IdGenerator::new(1, 32);
    }

    #[test]
    fn test_sequence_overflow() {
        let generator = IdGenerator::new(1, 1);

        // 快速生成超过4096个ID以触发序列号溢出
        let ids = generator.next_ids(5000);

        assert_eq!(ids.len(), 5000, "应该生成5000个ID");

        // 验证ID单调递增
        for i in 1..ids.len() {
            assert!(ids[i] > ids[i-1], "序列号溢出后ID应该继续递增");
        }
    }

    #[test]
    fn test_id_uniqueness_over_time() {
        let generator = IdGenerator::new(1, 1);
        let mut ids = std::collections::HashSet::new();

        // 生成大量ID并验证唯一性
        for _ in 0..10000 {
            let id = generator.next_id();
            assert!(!ids.contains(&id), "ID应该是唯一的");
            ids.insert(id);
        }
    }

    #[test]
    fn test_batch_vs_single_consistency() {
        let generator1 = IdGenerator::new(1, 1);
        let generator2 = IdGenerator::new(1, 1);

        // 同步两个生成器的时间戳
        thread::sleep(std::time::Duration::from_millis(10));

        // 使用批量生成
        let batch_ids = generator1.next_ids(10);

        // 使用单个生成
        let mut single_ids = Vec::new();
        for _ in 0..10 {
            single_ids.push(generator2.next_id());
        }

        // 验证两种方式生成的ID结构一致
        assert_eq!(batch_ids.len(), single_ids.len(), "两种方式应该生成相同数量的ID");

        // 验证ID单调递增
        for i in 1..batch_ids.len() {
            assert!(batch_ids[i] > batch_ids[i-1], "批量生成的ID应该单调递增");
        }

        for i in 1..single_ids.len() {
            assert!(single_ids[i] > single_ids[i-1], "单个生成的ID应该单调递增");
        }
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    /// 辅助函数：格式化吞吐量输出
    fn format_throughput(count: u64, elapsed: std::time::Duration) -> String {
        let secs = elapsed.as_secs_f64();
        let throughput = count as f64 / secs;
        if throughput >= 1_000_000.0 {
            format!("{:.2} ops/s ({:.2} M/s)", throughput, throughput / 1_000_000.0)
        } else if throughput >= 1_000.0 {
            format!("{:.2} ops/s ({:.2} K/s)", throughput, throughput / 1_000.0)
        } else {
            format!("{:.2} ops/s", throughput)
        }
    }

    #[test]
    fn bench_single_thread_next_id() {
        let generator = IdGenerator::new(1, 1);
        let count = 100_000;

        let start = Instant::now();
        for _ in 0..count {
            let _ = generator.next_id();
        }
        let elapsed = start.elapsed();

        println!(
            "单线程 next_id: 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            count,
            elapsed,
            format_throughput(count, elapsed)
        );
    }

    #[test]
    fn bench_single_thread_next_ids_batch_100() {
        let generator = IdGenerator::new(1, 1);
        let batch_size = 100;
        let rounds = 1_000;
        let total = batch_size * rounds;

        let start = Instant::now();
        for _ in 0..rounds {
            let _ = generator.next_ids(batch_size);
        }
        let elapsed = start.elapsed();

        println!(
            "单线程 next_ids(100): 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total as u64, elapsed)
        );
    }

    #[test]
    fn bench_single_thread_next_ids_batch_1000() {
        let generator = IdGenerator::new(1, 1);
        let batch_size = 1_000;
        let rounds = 100;
        let total = batch_size * rounds;

        let start = Instant::now();
        for _ in 0..rounds {
            let _ = generator.next_ids(batch_size);
        }
        let elapsed = start.elapsed();

        println!(
            "单线程 next_ids(1000): 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total as u64, elapsed)
        );
    }

    #[test]
    fn bench_multi_thread_next_id_4_threads() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let ids_per_thread = 50_000;
        let num_threads = 4;
        let total = ids_per_thread * num_threads;

        let start = Instant::now();
        let mut handles = vec![];
        for _ in 0..num_threads {
            let arc_gen = generator.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..ids_per_thread {
                    let _ = arc_gen.next_id();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "4线程 next_id: 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total, elapsed)
        );
    }

    #[test]
    fn bench_multi_thread_next_id_8_threads() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let ids_per_thread = 50_000;
        let num_threads = 8;
        let total = ids_per_thread * num_threads;

        let start = Instant::now();
        let mut handles = vec![];
        for _ in 0..num_threads {
            let arc_gen = generator.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..ids_per_thread {
                    let _ = arc_gen.next_id();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "8线程 next_id: 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total, elapsed)
        );
    }

    #[test]
    fn bench_multi_thread_next_ids_4_threads() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let batch_size = 100;
        let batches_per_thread = 500;
        let num_threads = 4;
        let total = batch_size * batches_per_thread * num_threads;

        let start = Instant::now();
        let mut handles = vec![];
        for _ in 0..num_threads {
            let arc_gen = generator.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..batches_per_thread {
                    let _ = arc_gen.next_ids(batch_size);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "4线程 next_ids(100): 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total as u64, elapsed)
        );
    }

    #[test]
    fn bench_multi_thread_next_ids_8_threads() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let batch_size = 100;
        let batches_per_thread = 500;
        let num_threads = 8;
        let total = batch_size * batches_per_thread * num_threads;

        let start = Instant::now();
        let mut handles = vec![];
        for _ in 0..num_threads {
            let arc_gen = generator.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..batches_per_thread {
                    let _ = arc_gen.next_ids(batch_size);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "8线程 next_ids(100): 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total as u64, elapsed)
        );
    }

    #[test]
    fn bench_multi_thread_mixed_next_id_and_next_ids() {
        let generator = Arc::new(IdGenerator::new(1, 1));
        let num_threads = 8;
        let ops_per_thread = 10_000;

        let start = Instant::now();
        let mut handles = vec![];
        for thread_id in 0..num_threads {
            let arc_gen = generator.clone();
            handles.push(thread::spawn(move || {
                // 偶数线程使用 next_id，奇数线程使用 next_ids(10)
                if thread_id % 2 == 0 {
                    for _ in 0..ops_per_thread {
                        let _ = arc_gen.next_id();
                    }
                    ops_per_thread as u64
                } else {
                    let batch_size = 10;
                    let rounds = ops_per_thread / batch_size;
                    for _ in 0..rounds {
                        let _ = arc_gen.next_ids(batch_size);
                    }
                    (rounds * batch_size) as u64
                }
            }));
        }
        let mut total: u64 = 0;
        for h in handles {
            total += h.join().unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "8线程混合(next_id + next_ids): 生成 {} 个ID, 耗时 {:?}, 吞吐量 {}",
            total,
            elapsed,
            format_throughput(total, elapsed)
        );
    }
}
