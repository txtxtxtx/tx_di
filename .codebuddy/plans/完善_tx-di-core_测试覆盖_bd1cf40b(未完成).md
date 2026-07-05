---
name: 完善 tx-di-core 测试覆盖
overview: 补齐 tx-di-core 测试和基准测试的严重缺口，确保框架生命周期、错误路径、所有公共 API 均有覆盖
todos:
  - id: lifecycle-test-components
    content: 定义生命周期测试组件（含 init/app_init/async_init/async_run/shutdown 标记 + 全局计数器）
    status: pending
  - id: lifecycle-tests
    content: 实现生命周期钩子测试（inner_init / init_sort / async_init / shutdown 全链路）
    status: pending
    dependencies:
      - lifecycle-test-components
  - id: app-async-tests
    content: 实现 App 异步生命周期测试（ins_run / shutdown / build_and_run）
    status: pending
  - id: topo-error-tests
    content: 实现拓扑排序错误路径测试（循环依赖/依赖未注册）
    status: pending
  - id: store-edge-tests
    content: 实现 Store 边缘操作测试（insert_arc/insert_factory/into_inner/from_dashmap）
    status: pending
  - id: trait-bulk-test
    content: 实现批量 Trait 注入测试（inject_all_traits_from_store）
    status: pending
  - id: deps-resolve-test
    content: 实现 DepsTuple::resolve 从 Store 实际解析的测试
    status: pending
  - id: config-file-test
    content: 实现配置组件真实 TOML 文件加载测试
    status: pending
  - id: benchmarks
    content: 补充基准测试（inject_from_store / inject_trait_from_store / App build 性能）
    status: pending
  - id: verify
    content: cargo build + cargo test + cargo bench 编译验证全部通过
    status: pending
    dependencies:
      - lifecycle-tests
      - app-async-tests
      - topo-error-tests
      - store-edge-tests
      - trait-bulk-test
      - deps-resolve-test
      - config-file-test
      - benchmarks
---

## 测试覆盖完整性分析

### 当前测试（38 个测试通过）覆盖了 13 个方面：

1. 基础注入（无依赖/单依赖/多依赖/深层链/大量依赖）
2. 作用域（Singleton/Prototype 隔离性）
3. 字段属性（tx_cst 自定义值/skip/Option）
4. Trait Object 注入（consumer 字段/store 直接获取）
5. 配置组件（默认值路径）
6. Store 操作（contains/len/try_inject/inject 错误）
7. BuildContext & App（构建/注入/访问）
8. 跨组件一致性（共享依赖同一实例）
9. AOP 拦截器（before/after/chain/logging/metrics）
10. 错误处理（未注册 panic）
11. 并发注入（8 线程 Singleton/Prototype）
12. DepsTuple 静态方法（dep_type_ids）
13. Scope 枚举（is_singleton/is_prototype/default）

### 基准测试覆盖了 5 个方面：

14. 拓扑排序（无依赖/链式/10/50/100 规模）
15. 依赖注入底层（Singleton 小/大对象/Prototype/lookup_miss/多 key）
16. 并发 DashMap（只读/读写混合/2/4/8 线程）
17. CompRef 开销（cached clone/downcast/factory call/DashMap ops）
18. 异步基础（tokio_spawn/cancellation/arc_clone）

### 已发现的缺口

**严重缺口（critical）：**

- 生命周期钩子全部未测试：LifecycleComponent 定义了但从未使用，inner_init/init/async_init/async_run/shutdown 回调均无测试
- init_sort 排序逻辑未验证
- App 异步生命周期：App::ins_run() / shutdown() / BuildContext::build_and_run() 无测试
- 拓扑排序错误路径：循环依赖和依赖未注册无测试

**中等缺口：**

- Store 边缘操作：insert_arc/insert_factory/into_inner/from_dashmap 无测试
- inject_all_traits_from_store() 无测试
- DepsTuple::resolve() 实际解析未测试（只测了 dep_type_ids）
- 配置组件真实文件路径未测试

**基准缺口：**

- 高层注入函数 inject_from_store / inject_trait_from_store 性能
- AOP InterceptorChain 开销
- App::build 构建性能

## 目标

补齐上述所有缺口，使测试覆盖率达到框架全部公共 API 的程度，避免严重 bug 漏掉。同时保持代码风格与现有测试一致。

## 技术方案

### 测试策略

所有新增测试遵循现有文件的风格：

- 放在 `tx-di-core/tests/test_component.rs` 中（集成测试）
- 使用 `#[derive(Component)]` 宏定义测试组件
- 使用 `BuildContext::new::<PathBuf>(None)` 自动注册
- 使用 `std::sync::Arc` + 全局静态变量验证回调调用
- 拓扑排序错误路径使用 `topology::topo_sort()` 直接测试（不经过 BuildContext），避免编译期 linkme 注册影响

### 生命周期钩子测试设计

关键问题：init/async_init/async_run/shutdown 回调在 App 阶段运行，需要 tokio runtime。测试方案：

1. **inner_init 钩子**：直接在 BuildContext 构建后验证（无需 App 阶段）
2. **init/app_init 钩子**：通过 `App::init()` 触发（`App` 的 `init` 是私有方法...） 实际上 `App` 的 `init` 是 `fn init(app: &Arc<App>)`，是私有关联函数。但 `ins_run()` 公开且会触发 init -> async_init -> async_run。对于单元测试可以使用 tokio::test 或 rt::block_on。
3. **async_init 钩子**：同样通过 `ins_run()` 触发
4. **shutdown 钩子**：`App::shutdown()` 是 `pub async fn`，可直接在测试中调用
5. **init_sort 排序**：定义两个 init_sort 值不同的组件（如 1 和 100），验证 init 阶段的执行顺序

### 各缺口测试实现方案

**生命周期钩子（6-8 个测试）：**

- 定义 AppConfig（已自动注册为配置组件）作为公共依赖
- 对每个生命周期标记定义不同的测试组件 + 全局计数器
- `test_inner_init_hook`: 验证 `init` 回调在 `BuildContext::new()` 后被调用
- `test_app_shutdown`: 在 tokio runtime 中构造 App，调用 shutdown() 验证
- `test_lifecycle_full_flow`: build -> inner_init -> init -> async_init -> shutdown 全链路
- `test_init_sort_ordering`: EarlyInit(init_sort=1) 与 LateInit(init_sort=100)，验证 init 调用顺序

**App 异步生命周期（2-3 个测试）：**

- `test_ins_run`: 在 tokio runtime 中创建 App，调用 ins_run()，验证 async_init 钩子被调用，然后 shutdown
- `test_build_and_run`: BuildContext::build_and_run() 在 tokio runtime 中执行

**拓扑排序错误路径（2 个测试）：**

- 手动构造 ComponentMeta 列表（不需要 derive 宏），直接调用 `topo_sort()`
- `test_topo_sort_cycle`: A 依赖 B，B 依赖 A
- `test_topo_sort_missing_dep`: A 依赖一个不存在的 TypeId

**Store 边缘操作（2 个测试）：**

- 直接操作 `Store::new()`，不借助 BuildContext
- 使用简单的 `u64` 或 `String` 类型

**批量 Trait 注入（2 个测试）：**

- 定义一个新的 trait 和两个实现，均标注 `#[component(as_trait = dyn ...)]`
- 通过 `inject_all_traits_from_store` 获取 Vec 并验证两个都在

**DepsTuple::resolve（1 个测试）：**

- 手动将组件放到 Store 中，然后调用 `<(Arc<DbPool>, Arc<RedisClient>) as DepsTuple>::resolve(store)`

**配置组件真实文件（1 个测试）：**

- 使用 `std::env::temp_dir()` 创建临时 TOML 文件
- 通过 `BuildContext::new(Some(path))` 加载

**基准测试（2-3 项）：**

- `inject_from_store` + `inject_trait_from_store` 高层 API 基准
- `App::build` 构建速度
- 用宏自动生成无依赖组件 + 注入循环

### 注意事项

1. 生命周期测试需要 tokio runtime：使用 `tokio::runtime::Runtime::new().unwrap().block_on(...)`
2. `LifecycleComponent` 当前的 static 计数器和组件定义可以复用，但需要加上生命周期标记
3. 不同测试文件中引入的组件会通过 linkme 注册到同一个 `COMPONENT_REGISTRY`，因此不同测试文件间可能有副作用。但 `test_component.rs` 是唯一集成测试文件，在同一进程中运行，这个影响是可控的。
4. 拓扑排序错误路径测试不经过 BuildContext，直接调用 `topo_sort`，不会受 linkme 注册的组件影响（传入的是手动构造的列表）
5. 基准测试中 `criterion_group!(benches, ...)` 需要注册新函数

## 技能使用

### rust-ddd-test-generator

本计划涉及的测试覆盖分析、测试代码生成、基准测试设计属于 Rust 项目的测试增强场景，`rust-ddd-test-generator` skill 可以提供测试组织模式参考和测试用例模板建议，用于确保所有公共 API 被充分测试。

### code-explorer

使用 SubAgent 进行代码探索已经在准备阶段完成。执行阶段不再需要。