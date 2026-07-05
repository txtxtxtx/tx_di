//! tx-di-core 性能基准测试
//!
//! ```shell
//! cargo bench -p tx-di-core --bench di_bench
//! ```
//! 覆盖场景：
//! 1. 拓扑排序 (topo_sort) — 不同规模和依赖深度
//! 2. 依赖注入 (inject) — Singleton 缓存 vs Prototype 工厂
//! 3. 并发注入 — 多线程 DashMap 读写竞争
//! 4. CompRef 克隆 / downcast / DashMap 操作
//! 5. 异步运行时开销
//! 6. 高层注入函数 (inject_from_store / inject_trait_from_store)
//! 7. App::build 构建性能

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::any::{Any, TypeId};
use std::path::PathBuf;
use std::sync::Arc;
use std::hint::black_box;
use tx_di_core::{
    BoxFuture, BuildContext, CancellationToken, CompRef, Component, ComponentMeta, DepsTuple, RIE,
    Scope, Store, TraitImplMap, inject_from_store, topo_sort,
};

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

// ── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 空的 init fn
fn noop_init(_app: &Arc<tx_di_core::App>) -> RIE<()> { Ok(()) }
/// 空的 async_init fn
fn noop_async_init(_app: &Arc<tx_di_core::App>) -> BoxFuture<RIE<()>> {
    Box::pin(async { Ok(()) })
}
/// 空的 async_run fn
fn noop_async_run(_app: &Arc<tx_di_core::App>, _token: CancellationToken) -> BoxFuture<RIE<()>> {
    Box::pin(async { Ok(()) })
}
/// 空的 shutdown fn
fn noop_shutdown(_store: &Store) {}
/// init_sort 返回 0
fn zero_sort() -> i32 { 0 }

/// 空的 factory fn（返回空 Box）
fn noop_factory(_store: &Store) -> Box<dyn Any + Send + Sync> {
    Box::new(())
}

/// 构建 n 个无依赖的 ComponentMeta
fn make_independent_metas(n: usize) -> Vec<ComponentMeta> {
    (0..n)
        .map(|i| {
            let type_id_fn = MARKER_FNS[i % MARKER_FNS.len()];
            ComponentMeta {
                type_id: type_id_fn,
                name: "bench",
                dep_type_ids: &[],
                factory: noop_factory,
                scope: Scope::Singleton,
                impl_traits: &[],
                trait_impls: &[],
                init_sort_fn: zero_sort,
                init_fn: noop_init,
                async_init_fn: noop_async_init,
                async_run_fn: noop_async_run,
                shutdown_fn: noop_shutdown,
            }
        })
        .collect()
}

/// 构建线性依赖链：dep[i] = type_id[i-1]
fn make_chain_metas(n: usize) -> Vec<ComponentMeta> {
    assert!(n <= MARKER_FNS.len(), "chain length exceeds marker count");

    let type_ids: Vec<fn() -> TypeId> = (0..n).map(|i| MARKER_FNS[i]).collect();

    let mut deps_vecs: Vec<&'static [fn() -> TypeId]> = Vec::with_capacity(n);
    for i in 0..n {
        if i == 0 {
            deps_vecs.push(&[]);
        } else {
            let dep_fn = type_ids[i - 1];
            let leaked: &'static [fn() -> TypeId] = Box::leak(Box::new([dep_fn]));
            deps_vecs.push(leaked);
        }
    }

    (0..n)
        .map(|i| ComponentMeta {
            type_id: type_ids[i],
            name: "bench",
            dep_type_ids: deps_vecs[i],
            factory: noop_factory,
            scope: Scope::Singleton,
            impl_traits: &[],
            trait_impls: &[],
            init_sort_fn: zero_sort,
            init_fn: noop_init,
            async_init_fn: noop_async_init,
            async_run_fn: noop_async_run,
            shutdown_fn: noop_shutdown,
        })
        .collect()
}

/// 快速构建 Store，放入一个 Cached 实例
fn make_store_with<T: Any + Send + Sync + 'static>(value: T) -> Store {
    let store = Store::new();
    store.insert_cached(value);
    store
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 1: 拓扑排序
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_topo_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("topo_sort");
    // 拓扑排序需要一个 TraitImplMap 引用，所有基准组件都不涉及 trait 依赖
    let empty_trait_impls = TraitImplMap::new();

    for &n in &[10usize, 50, 100] {
        let label = format!("no_deps_{n}");
        group.bench_function(&label, |b| {
            let metas = make_independent_metas(n);
            let refs: Vec<&ComponentMeta> = metas.iter().collect();
            b.iter(|| {
                let _ = black_box(topo_sort(black_box(&refs), &empty_trait_impls));
            });
        });
    }

    for &n in &[10usize, 50, 100] {
        let label = format!("chain_{n}");
        group.bench_function(&label, |b| {
            let metas = make_chain_metas(n);
            let refs: Vec<&ComponentMeta> = metas.iter().collect();
            b.iter(|| {
                let _ = black_box(topo_sort(black_box(&refs), &empty_trait_impls));
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

    // 2.1 Singleton 注入（小对象）
    group.bench_function("singleton_u64", |b| {
        let store = make_store_with(42u64);
        b.iter(|| {
            let store = store.inner();
            let tid = TypeId::of::<u64>();
            if let Some(entry) = store.get(&tid) {
                if let CompRef::Cached(arc) = &*entry {
                    let _ = arc.clone();
                }
            }
            black_box(());
        });
    });

    // 2.2 Singleton 注入（大对象）
    group.bench_function("singleton_large_object", |b| {
        let large = LargeObject {
            _data: vec![0u8; 4096],
            _name: "bench".to_string(),
            _count: 12345,
        };
        let store = make_store_with(large);
        b.iter(|| {
            let store = store.inner();
            let tid = TypeId::of::<LargeObject>();
            if let Some(entry) = store.get(&tid) {
                if let CompRef::Cached(arc) = &*entry {
                    let _ = arc.clone();
                }
            }
            black_box(());
        });
    });

    // 2.3 Prototype 注入（每次调用工厂闭包）
    group.bench_function("prototype_factory", |b| {
        let store = Store::new();
        let closure = |_store: &Store| -> Arc<dyn Any + Send + Sync> {
            Arc::new(0u64)
        };
        store.inner().insert(
            TypeId::of::<u64>(),
            CompRef::Factory(Arc::new(closure)),
        );

        b.iter(|| {
            let tid = TypeId::of::<u64>();
            if let Some(entry) = store.inner().get(&tid) {
                if let CompRef::Factory(f) = &*entry {
                    let _ = f(&store);
                }
            }
            black_box(());
        });
    });

    // 2.4 注入 miss（key 不存在时的 get 开销）
    group.bench_function("lookup_miss", |b| {
        let store = Store::new();
        b.iter(|| {
            let tid = TypeId::of::<u64>();
            let result = store.inner().get(&tid);
            black_box(result);
        });
    });

    // 2.5 多类型 store 中查找（10 个 key）
    group.bench_function("multi_type_10_keys", |b| {
        let store = Store::new();
        store.insert_cached(1u8);
        store.insert_cached(2u16);
        store.insert_cached(3u32);
        store.insert_cached(4u64);
        store.insert_cached(5i8);
        store.insert_cached(6i16);
        store.insert_cached(7i32);
        store.insert_cached(8i64);
        store.insert_cached(9.0f32);
        store.insert_cached(10.0f64);

        b.iter(|| {
            let tid = TypeId::of::<u64>();
            if let Some(entry) = store.inner().get(&tid) {
                if let CompRef::Cached(arc) = &*entry {
                    let _ = arc.clone();
                }
            }
            black_box(());
        });
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 3: 并发 DashMap 访问
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");

    // 3.1 并发读
    for &tc in &[2u32, 4, 8] {
        group.bench_function(format!("read_only/{tc}_threads"), |b| {
            let store = Arc::new(make_store_with(42u64));
            b.iter(|| {
                let mut handles = Vec::with_capacity(tc as usize);
                for _ in 0..tc {
                    let store = store.clone();
                    handles.push(std::thread::spawn(move || {
                        for _ in 0..1000 {
                            let inner = store.inner();
                            let tid = TypeId::of::<u64>();
                            if let Some(entry) = inner.get(&tid) {
                                if let CompRef::Cached(arc) = &*entry {
                                    let _ = arc.clone();
                                }
                            }
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    // 3.2 读写混合
    for &tc in &[2u32, 4, 8] {
        group.bench_function(format!("read_write/{tc}_threads"), |b| {
            b.iter_batched(
                || Arc::new(make_store_with(0u64)),
                |store| {
                    let mut handles = Vec::new();

                    for _ in 0..tc {
                        let store = store.clone();
                        handles.push(std::thread::spawn(move || {
                            for _ in 0..500 {
                                let inner = store.inner();
                                let tid = TypeId::of::<u64>();
                                if let Some(entry) = inner.get(&tid) {
                                    if let CompRef::Cached(arc) = &*entry {
                                        let _ = arc.clone();
                                    }
                                }
                            }
                        }));
                    }

                    for i in 0..tc {
                        let store = store.clone();
                        handles.push(std::thread::spawn(move || {
                            for _ in 0..100 {
                                store.insert_cached(i as u64);
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

    group.bench_function("cached_clone", |b| {
        let comp_ref = CompRef::Cached(Arc::new(42u64));
        b.iter(|| {
            let cloned = comp_ref.clone();
            black_box(cloned);
        });
    });

    group.bench_function("cached_downcast", |b| {
        let comp_ref = CompRef::Cached(Arc::new(42u64));
        b.iter(|| {
            if let CompRef::Cached(arc) = &comp_ref {
                let val = arc.clone().downcast::<u64>().unwrap();
                black_box(val);
            }
        });
    });

    group.bench_function("factory_call", |b| {
        let closure = |_store: &Store| -> Arc<dyn Any + Send + Sync> {
            Arc::new(0u64)
        };
        let comp_ref = CompRef::Factory(Arc::new(closure));
        let store = Store::new();

        b.iter(|| {
            if let CompRef::Factory(f) = &comp_ref {
                let val = f(&store);
                black_box(val);
            }
        });
    });

    group.bench_function("dashmap_insert_get", |b| {
        let store = Store::new();
        b.iter(|| {
            store.insert_cached(42u64);
            let _ = store.inner().get(&TypeId::of::<u64>());
        });
    });

    group.bench_function("dashmap_get_existing", |b| {
        let store = Store::new();
        store.insert_cached(42u64);
        b.iter(|| {
            let _ = store.inner().get(&TypeId::of::<u64>());
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

    group.bench_function("cancellation_token_clone", |b| {
        let token = CancellationToken::new();
        b.iter(|| {
            let cloned = token.clone();
            black_box(cloned);
        });
    });

    group.bench_function("cancellation_check", |b| {
        let token = CancellationToken::new();
        b.iter(|| {
            black_box(token.is_cancelled());
        });
    });

    for &tc in &[1u32, 2, 4] {
        let store = Arc::new(make_store_with(42u64));
        group.bench_function(format!("arc_clone/{tc}_threads"), |b| {
            b.iter(|| {
                let mut handles = Vec::with_capacity(tc as usize);
                for _ in 0..tc {
                    let store = store.clone();
                    handles.push(std::thread::spawn(move || {
                        for _ in 0..1000 {
                            let inner = store.inner();
                            let tid = TypeId::of::<u64>();
                            if let Some(entry) = inner.get(&tid) {
                                if let CompRef::Cached(arc) = &*entry {
                                    let _ = arc.clone();
                                }
                            }
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
// Benchmark Group 6: 高层注入函数
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 用于基准测试的组件类型
#[derive(Component, Default)]
pub struct BenchComp;

fn bench_inject_high_level(c: &mut Criterion) {
    let mut group = c.benchmark_group("inject_high_level");

    group.bench_function("inject_from_store", |b| {
        let store = Store::new();
        store.insert_cached(BenchComp);
        b.iter(|| {
            let val = inject_from_store::<BenchComp>(black_box(&store));
            black_box(val);
        });
    });

    group.bench_function("inject_from_store_multi", |b| {
        let store = Store::new();
        store.insert_cached(BenchComp);
        store.insert_cached(BenchComp);
        store.insert_cached(BenchComp);
        b.iter(|| {
            let a = inject_from_store::<BenchComp>(&store);
            let b = inject_from_store::<BenchComp>(&store);
            let c = inject_from_store::<BenchComp>(&store);
            black_box((a, b, c));
        });
    });

    // Prototype: bench inject_from_store with factory
    group.bench_function("inject_from_store_factory", |b| {
        let store = Store::new();
        store.insert_factory::<BenchComp, _>(|_| Arc::new(BenchComp));
        b.iter(|| {
            let val = inject_from_store::<BenchComp>(black_box(&store));
            black_box(val);
        });
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Benchmark Group 7: App 构建
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_app_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("app_build");

    group.bench_function("build_empty", |b| {
        b.iter_batched(
            || BuildContext::new::<PathBuf>(None),
            |ctx| {
                let app = ctx.build().unwrap();
                black_box(app);
            },
            BatchSize::SmallInput,
        );
    });

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
    bench_inject_high_level,
    bench_app_build,
);
criterion_main!(benches);
