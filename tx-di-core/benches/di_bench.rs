//! tx-di-core 性能基准测试
//!
//! 覆盖场景：
//! 1. 拓扑排序 (topo_sort) — 不同规模和依赖深度
//! 2. 依赖注入 (inject) — Singleton 缓存 vs Prototype 工厂
//! 3. 并发注入 — 多线程 DashMap 读写竞争
//! 4. CompRef 克隆 / downcast / DashMap 操作
//! 5. 异步运行时开销

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::sync::Arc;
use tx_di_core::{CompRef, ComponentMeta, Scope, inject_from_store, topo_sort};

// ── 生成唯一 marker 类型 ─────────────────────────────────────────────────────

macro_rules! define_marker_types {
    ($($name:ident),+ $(,)?) => {
        $(struct $name;)+
        const MARKER_FNS: &[fn() -> TypeId] = &[$(|| TypeId::of::<$name>()),+];
    };
}

define_marker_types!(
    M0, M1, M2, M3, M4, M5, M6, M7, M8, M9,
    M10, M11, M12, M13, M14, M15, M16, M17, M18, M19,
    M20, M21, M22, M23, M24, M25, M26, M27, M28, M29,
    M30, M31, M32, M33, M34, M35, M36, M37, M38, M39,
    M40, M41, M42, M43, M44, M45, M46, M47, M48, M49,
    M50, M51, M52, M53, M54, M55, M56, M57, M58, M59,
    M60, M61, M62, M63, M64, M65, M66, M67, M68, M69,
    M70, M71, M72, M73, M74, M75, M76, M77, M78, M79,
    M80, M81, M82, M83, M84, M85, M86, M87, M88, M89,
    M90, M91, M92, M93, M94, M95, M96, M97, M98, M99,
);

// ── 辅助类型 ─────────────────────────────────────────────────────────────────

struct LargeObject {
    _data: Vec<u8>,
    _name: String,
    _count: u64,
}

/// init_sort_fn 实现：固定返回 0
fn zero_sort() -> i32 {
    0
}

// ── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 构建 n 个无依赖的 ComponentMeta
/// 用静态 fn 指针数组 MARKER_FNS 循环分配唯一 TypeId
fn make_independent_metas(n: usize) -> Vec<ComponentMeta> {
    (0..n)
        .map(|i| {
            let type_id_fn = MARKER_FNS[i % MARKER_FNS.len()];
            ComponentMeta {
                type_id: type_id_fn,
                deps: &[],
                name: "bench",
                scope: Scope::Singleton,
                factory_fn: None,
                init_sort_fn: zero_sort,
                init_fn: None,
                async_init_fn: None,
                async_run_fn: None,
                impl_traits: &[],
                trait_impls: &[],
            }
        })
        .collect()
}

/// 构建线性依赖链：dep[i] = type_id[i-1]
/// 这里用 static 数组存储 deps 的函数指针
fn make_chain_metas(n: usize) -> Vec<ComponentMeta> {
    assert!(n <= MARKER_FNS.len(), "chain length exceeds marker count");

    let type_ids: Vec<fn() -> TypeId> = (0..n)
        .map(|i| MARKER_FNS[i])
        .collect();

    // 构建 deps: meta[i] 依赖 type_ids[i-1]
    // 由于 deps 是 &'static [fn() -> TypeId]，需要 leak 或者用全局
    // 这里为每个 meta 单独 leak 一个小数组
    let mut deps_vecs: Vec<&'static [fn() -> TypeId]> = Vec::with_capacity(n);
    for i in 0..n {
        if i == 0 {
            deps_vecs.push(&[]);
        } else {
            // leak 一个包含单个 fn 指针的 slice
            let dep_fn = type_ids[i - 1];
            let leaked: &'static mut [fn() -> TypeId] =
                Box::leak(Box::new([dep_fn]));
            deps_vecs.push(leaked);
        }
    }

    (0..n)
        .map(|i| {
            ComponentMeta {
                type_id: type_ids[i],
                deps: deps_vecs[i],
                name: "bench",
                scope: Scope::Singleton,
                factory_fn: None,
                init_sort_fn: zero_sort,
                init_fn: None,
                async_init_fn: None,
                async_run_fn: None,
                impl_traits: &[],
                trait_impls: &[],
            }
        })
        .collect()
}

/// 快速构建 DashMap store，放入一个 Cached 实例
fn make_store_with<T: Any + Send + Sync + 'static>(value: T) -> DashMap<TypeId, CompRef> {
    let store = DashMap::<TypeId, CompRef>::new();
    store.insert(
        TypeId::of::<T>(),
        CompRef::Cached(Arc::new(value) as Arc<dyn Any + Send + Sync>),
    );
    store
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 1: 拓扑排序
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_topo_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("topo_sort");

    // ── 1.1 无依赖拓扑排序（纯 O(V+E) 开销） ──
    for &n in &[10usize, 50, 100] {
        let label = format!("no_deps_{n}");
        group.bench_function(&label, |b| {
            let metas = make_independent_metas(n);
            let refs: Vec<&ComponentMeta> = metas.iter().collect();
            b.iter(|| {
                black_box(topo_sort(black_box(&refs)));
            });
        });
    }

    // ── 1.2 链式拓扑排序（深度 = n） ──
    for &n in &[10usize, 50, 100] {
        let label = format!("chain_{n}");
        group.bench_function(&label, |b| {
            let metas = make_chain_metas(n);
            let refs: Vec<&ComponentMeta> = metas.iter().collect();
            b.iter(|| {
                black_box(topo_sort(black_box(&refs)));
            });
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 2: 依赖注入 (inject)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_inject(c: &mut Criterion) {
    let mut group = c.benchmark_group("inject");

    // ── 2.1 Singleton 注入（从 Cached 取小对象） ──
    group.bench_function("singleton_u64", |b| {
        let store = make_store_with(42u64);
        b.iter(|| {
            let val = inject_from_store::<u64>(black_box(&store));
            black_box(val);
        });
    });

    // ── 2.2 Singleton 注入（大对象） ──
    group.bench_function("singleton_large_object", |b| {
        let large = LargeObject {
            _data: vec![0u8; 4096],
            _name: "bench".to_string(),
            _count: 12345,
        };
        let store = make_store_with(large);
        b.iter(|| {
            let val = inject_from_store::<LargeObject>(black_box(&store));
            black_box(val);
        });
    });

    // ── 2.3 Prototype 注入（每次调用工厂闭包） ──
    group.bench_function("prototype_factory", |b| {
        let store = DashMap::<TypeId, CompRef>::new();
        let closure = |_store: &DashMap<TypeId, CompRef>| -> Arc<dyn Any + Send + Sync> {
            Arc::new(0u64)
        };
        store.insert(TypeId::of::<u64>(), CompRef::Factory(Arc::new(closure)));

        b.iter(|| {
            let val = inject_from_store::<u64>(black_box(&store));
            black_box(val);
        });
    });

    // ── 2.4 注入 miss（key 不存在时的 get 开销） ──
    group.bench_function("lookup_miss", |b| {
        let store = DashMap::<TypeId, CompRef>::new();
        b.iter(|| {
            let tid = TypeId::of::<u64>();
            let result = store.get(&tid);
            black_box(result);
        });
    });

    // ── 2.5 多类型 store 中查找（10 个 key） ──
    group.bench_function("multi_type_10_keys", |b| {
        let store = DashMap::<TypeId, CompRef>::new();
        store.insert(TypeId::of::<u8>(), CompRef::Cached(Arc::new(1u8)));
        store.insert(TypeId::of::<u16>(), CompRef::Cached(Arc::new(2u16)));
        store.insert(TypeId::of::<u32>(), CompRef::Cached(Arc::new(3u32)));
        store.insert(TypeId::of::<u64>(), CompRef::Cached(Arc::new(4u64)));
        store.insert(TypeId::of::<i8>(), CompRef::Cached(Arc::new(5i8)));
        store.insert(TypeId::of::<i16>(), CompRef::Cached(Arc::new(6i16)));
        store.insert(TypeId::of::<i32>(), CompRef::Cached(Arc::new(7i32)));
        store.insert(TypeId::of::<i64>(), CompRef::Cached(Arc::new(8i64)));
        store.insert(TypeId::of::<f32>(), CompRef::Cached(Arc::new(9.0f32)));
        store.insert(TypeId::of::<f64>(), CompRef::Cached(Arc::new(10.0f64)));

        b.iter(|| {
            let val = inject_from_store::<u64>(black_box(&store));
            black_box(val);
        });
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 3: 并发 DashMap 访问
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");

    // ── 3.1 并发读（多线程读同一 key） ──
    for &tc in &[2u32, 4, 8] {
        group.bench_function(format!("read_only/{tc}_threads"), |b| {
            let store = Arc::new(make_store_with(42u64));
            b.iter(|| {
                let mut handles = Vec::with_capacity(tc as usize);
                for _ in 0..tc {
                    let store = store.clone();
                    handles.push(std::thread::spawn(move || {
                        for _ in 0..1000 {
                            let val = inject_from_store::<u64>(&store);
                            black_box(val);
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    // ── 3.2 读写混合 ──
    for &tc in &[2u32, 4, 8] {
        group.bench_function(format!("read_write/{tc}_threads"), |b| {
            b.iter_batched(
                || Arc::new(make_store_with(0u64)),
                |store| {
                    let mut handles = Vec::new();

                    // 读线程
                    for _ in 0..tc {
                        let store = store.clone();
                        handles.push(std::thread::spawn(move || {
                            for _ in 0..500 {
                                let val = inject_from_store::<u64>(&store);
                                black_box(val);
                            }
                        }));
                    }

                    // 写线程
                    for i in 0..tc {
                        let store = store.clone();
                        handles.push(std::thread::spawn(move || {
                            for _ in 0..100 {
                                store.insert(
                                    TypeId::of::<u64>(),
                                    CompRef::Cached(Arc::new(i as u64)),
                                );
                            }
                        }));
                    }

                    for h in handles {
                        h.join().unwrap();
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 4: CompRef 开销
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_comp_ref(c: &mut Criterion) {
    let mut group = c.benchmark_group("comp_ref");

    // ── 4.1 Cached clone ──
    group.bench_function("cached_clone", |b| {
        let comp_ref = CompRef::Cached(Arc::new(42u64));
        b.iter(|| {
            let cloned = comp_ref.clone();
            black_box(cloned);
        });
    });

    // ── 4.2 Cached downcast ──
    group.bench_function("cached_downcast", |b| {
        let comp_ref = CompRef::Cached(Arc::new(42u64));
        b.iter(|| {
            if let CompRef::Cached(arc) = &comp_ref {
                let val = arc.clone().downcast::<u64>().unwrap();
                black_box(val);
            }
        });
    });

    // ── 4.3 Factory 调用 ──
    group.bench_function("factory_call", |b| {
        let closure = |_store: &DashMap<TypeId, CompRef>| -> Arc<dyn Any + Send + Sync> {
            Arc::new(0u64)
        };
        let comp_ref = CompRef::Factory(Arc::new(closure));
        let store = DashMap::<TypeId, CompRef>::new();

        b.iter(|| {
            if let CompRef::Factory(f) = &comp_ref {
                let val = f(&store);
                black_box(val);
            }
        });
    });

    // ── 4.4 DashMap insert + get ──
    group.bench_function("dashmap_insert_get", |b| {
        let store = DashMap::<TypeId, CompRef>::new();
        b.iter(|| {
            store.insert(
                TypeId::of::<u64>(),
                CompRef::Cached(Arc::new(42u64)),
            );
            let _ = store.get(&TypeId::of::<u64>());
        });
    });

    // ── 4.5 DashMap get 预填充 ──
    group.bench_function("dashmap_get_existing", |b| {
        let store = DashMap::<TypeId, CompRef>::new();
        store.insert(
            TypeId::of::<u64>(),
            CompRef::Cached(Arc::new(42u64)),
        );
        b.iter(|| {
            let _ = store.get(&TypeId::of::<u64>());
        });
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 5: 异步运行时开销
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_async(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_runtime");

    let rt = tokio::runtime::Runtime::new().unwrap();

    // ── 5.1 tokio::spawn ──
    group.bench_function("tokio_spawn", |b| {
        b.iter(|| {
            rt.block_on(async {
                let handle = tokio::spawn(async {
                    black_box(42u64);
                });
                handle.await.unwrap();
            });
        });
    });

    // ── 5.2 CancellationToken clone ──
    group.bench_function("cancellation_token_clone", |b| {
        let token = tx_di_core::CancellationToken::new();
        b.iter(|| {
            let cloned = token.clone();
            black_box(cloned);
        });
    });

    // ── 5.3 CancellationToken check ──
    group.bench_function("cancellation_check", |b| {
        let token = tx_di_core::CancellationToken::new();
        b.iter(|| {
            black_box(token.is_cancelled());
        });
    });

    // ── 5.4 Arc clone contention ──
    for &tc in &[1u32, 2, 4] {
        let store = Arc::new(make_store_with(42u64));
        group.bench_function(format!("arc_clone/{tc}_threads"), |b| {
            b.iter(|| {
                let mut handles = Vec::with_capacity(tc as usize);
                for _ in 0..tc {
                    let store = store.clone();
                    handles.push(std::thread::spawn(move || {
                        for _ in 0..1000 {
                            let val = inject_from_store::<u64>(&store);
                            black_box(val);
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

criterion_group!(
    benches,
    bench_topo_sort,
    bench_inject,
    bench_concurrent,
    bench_comp_ref,
    bench_async,
);
criterion_main!(benches);
