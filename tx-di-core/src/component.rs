//! Component trait — 类型驱动的依赖注入核心
//!
//! 每个被 DI 管理的类型实现此 trait，用 associated type `Deps` 声明依赖。
//! `#[derive(Component)]` 宏自动生成实现。

use std::any::TypeId;
use std::future::Future;
use std::sync::Arc;

use crate::error::AppError;
use crate::scope::Scope;
use crate::store::Store;

/// 异步 Future 类型别名
pub type BoxFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send>>;

/// 组件 trait — 每个被 DI 管理的类型都实现此 trait
///
/// 用 `#[derive(Component)]` 自动生成（新宏），或手动实现。
///
/// # 核心设计
///
/// - `Deps` associated type 声明依赖，编译期类型可知
/// - `build()` 是纯函数，从依赖构建自身
/// - 生命周期钩子全部有默认实现
///
/// # 生命周期
///
/// 1. `build()` — 构造实例（由宏生成）
/// 2. `inner_init()` — build 后同步初始化（可选）
/// 3. `init()` — 同步初始化（可选）
/// 4. `async_init()` — 异步初始化（可选）
/// 5. `run()` — 异步运行，长期任务（可选）
/// 6. `shutdown()` — 优雅关闭（可选）
pub trait Component: Send + Sync + 'static {
    /// 依赖元组，编译期类型可知
    ///
    /// 例如：`type Deps = (Arc<DbPool>, Arc<AppConfig>);`
    ///
    /// 无依赖时用 `()`。
    type Deps: DepsTuple;

    /// 从依赖和 Store 构建组件实例
    ///
    /// 接收已解析的 Deps 元组和 Store 引用。
    /// Store 用于注入 trait object 依赖（`Arc<dyn Trait>` / `Option<Arc<dyn Trait>>` / `Vec<Arc<dyn Trait>>`）。
    fn build(deps: Self::Deps, store: &Store) -> Self;

    /// 作用域，默认 Singleton
    const SCOPE: Scope = Scope::Singleton;

    // ── 生命周期钩子（全部有默认实现）─────────────────────────────────

    /// build 之后、init 之前调用（同步初始化）
    ///
    /// 可以访问 Store 注入额外依赖，但主要依赖应通过 `Deps` 声明。
    #[allow(unused_variables)]
    fn inner_init(&mut self, store: &Store) -> crate::RIE<()> {
        Ok(())
    }

    /// 同步初始化（在 App 阶段调用）
    ///
    /// 可以访问整个 App，用于跨组件协作初始化。
    #[allow(unused_variables)]
    fn init(app: &Arc<crate::App>) -> crate::RIE<()> {
        Ok(())
    }

    /// 异步初始化（在 tokio runtime 里调用）
    #[allow(unused_variables)]
    fn async_init(app: &Arc<crate::App>) -> BoxFuture<crate::RIE<()>> {
        Box::pin(async { Ok(()) })
    }

    /// 异步运行（在独立 task 里调用，直到 CancellationToken 触发）
    #[allow(unused_variables)]
    fn async_run(app: &Arc<crate::App>, token: crate::CancellationToken) -> BoxFuture<crate::RIE<()>> {
        Box::pin(async { Ok(()) })
    }

    /// 优雅关闭
    #[allow(unused_variables)]
    fn shutdown(&self) {}

    /// 初始化排序（值越小越先执行，默认 10000）
    fn init_sort() -> i32 {
        10000
    }

    /// 返回此组件实现的 trait TypeId 列表
    ///
    /// 由 `#[component(as_trait = ...)]` 宏自动生成。
    /// 默认为空 — 不实现任何 trait。
    fn trait_impls() -> &'static [fn() -> TypeId] {
        &[]
    }
}

/// 依赖元组 trait — 用宏为不同元数自动实现
///
/// 从 Store 解析所有依赖，返回元组。
pub trait DepsTuple: Sized {
    /// 从 Store 解析所有依赖
    fn resolve(store: &Store) -> Result<Self, AppError>;

    /// 返回依赖的 TypeId 列表（用于拓扑排序）
    fn dep_type_ids() -> Vec<TypeId>;
}

// ── 为元组自动实现 DepsTuple ──────────────────────────────────────────────

impl DepsTuple for () {
    fn resolve(_store: &Store) -> crate::RIE<Self> {
        Ok(())
    }

    fn dep_type_ids() -> Vec<TypeId> {
        Vec::new()
    }
}

macro_rules! impl_deps_tuple {
    ($($T:ident),+) => {
        impl<$($T: Component),+> DepsTuple for ($(Arc<$T>,)+) {
            fn resolve(store: &Store) -> Result<Self, AppError> {
                Ok(($(
                    store.inject::<$T>()?
                ,)+))
            }

            fn dep_type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$T>()),+]
            }
        }
    };
}

impl_deps_tuple!(A);
impl_deps_tuple!(A, B);
impl_deps_tuple!(A, B, C);
impl_deps_tuple!(A, B, C, D);
impl_deps_tuple!(A, B, C, D, E);
impl_deps_tuple!(A, B, C, D, E, F);
impl_deps_tuple!(A, B, C, D, E, F, G);
impl_deps_tuple!(A, B, C, D, E, F, G, H);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_deps_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
