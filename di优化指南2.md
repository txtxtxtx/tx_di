# tx-di DI 框架缺陷分析报告

> 分析范围：`tx-di-core` + `tx-di-macros` 两个核心 crate，基于 2026-07-25 源码。

---

## 一、严重缺陷（阻塞性 / 安全风险）

### 1.1 `unsafe` 必选 trait 注入缺少错误路径

**位置**：`tx-di-macros/src/codegen/component_impl.rs:65-67` + `inner_init.rs:41-46`

**问题**：`Arc<dyn Trait>` 必选 trait 注入（`TraitInjectRequired`）使用 `mem::zeroed()` 创建占位值，然后在 `inner_init` 中用 `ptr::write` 覆盖。但如果 `inject_trait_from_store` 在运行时 panic（比如找不到实现），则零值 `Arc` 永远不会被 drop。更严重的是，如果 `inner_init` 执行到一半 panic，`ptr::write` 可能未执行，结构体析构时会尝试 drop 一个零值 `Arc<dyn Trait>`，这会导致 UB（undefined behavior）。

**建议**：
- 在 `inner_init` 中为 `TraitInjectRequired` 字段添加 panic-safe 包装，或用 `ManuallyDrop` + 显式初始化保证安全
- 改为在 build 阶段直接注入（而非零值占位 + 覆盖），消除 unsafe

### 1.2 泛型结构体完全不可用

**位置**：`tx-di-macros/src/codegen/mod.rs:66-71` + `attr/comp_attr.rs:220-224`

**问题**：`#[component(for(...))]` 参数已经被解析但**仅丢弃内容**（`let _content: syn::ExprParen = input.parse()?;`），没有任何代码生成逻辑。而泛型结构体会被直接拒绝（错误提示让用户写 `for(...)`，写完后实际不生效）。这是一个**死胡同 API**。

**建议**：要么实现 `for(...)` 单态化支持，要么移除该解析逻辑并给出明确的"暂不支持"错误消息。

---

## 二、运行时 Panic 风险（生产环境隐患）

### 2.1 Mutex Poisoning 传播

**位置**：`tx-di-core/src/aop.rs:218,223`

**问题**：`get_interceptor_chain` 和 `set_interceptor_chain` 使用 `chains_map().lock().unwrap()`，若持有锁的线程 panic，Mutex 被 poison，后续所有调用都会 panic。在高并发场景下，一个线程的 panic 会级联影响所有拦截器操作。

**建议**：使用 `lock().unwrap_or_else(|poisoned| poisoned.into_inner())` 恢复 poisoned 锁，或改用 `RwLock` / `DashMap` 替代 `Mutex<HashMap>`。

### 2.2 拓扑排序 panic 传播

**位置**：`tx-di-core/src/lifecycle.rs:104-106`

**问题**：`topo_sort` 返回 `RIE<Vec<TypeId>>`（Result），但 `auto_register_all` 中调用 `topo_sort(...).unwrap_or_else(|e| panic!("{}", e));`，将可恢复错误转为不可恢复 panic。

**建议**：传播错误而非 panic，让调用方决定如何处理。

### 2.3 配置加载失败 panic

**位置**：`tx-di-core/src/config.rs:27-32,37-43,67-74,76-83`

**问题**：3 处 `unwrap_or_else(panic!)` — 获取可执行文件路径失败、获取可执行文件父目录失败、配置文件读取失败、配置文件解析失败，全部 panic。这意味着如果部署环境没有标准 `current_exe()` 路径或配置文件损坏，整个应用直接崩溃。

**建议**：将配置加载改为返回 `Result`，由调用方决定是 panic 还是降级到默认配置。

### 2.4 `inject_trait_from_store` 的多处 panic

**位置**：`tx-di-core/src/store.rs:221-226, 233-241`

**问题**：trait 的具体实现未注册时 panic；trait 无任何实现时 panic。这两个场景在运行时可能发生（如条件编译或插件未正确链接），应返回 `Result` 而非直接崩溃。

### 2.5 "可选" trait 注入并不可选

**位置**：`tx-di-macros/src/codegen/meta_entry.rs:40-46` + `inner_init.rs:29` + `topology.rs:61-70`

**问题**：
1. `trait_inject_fields`（`Option<Arc<dyn Trait>>`）和 `list_trait_fields`（`Vec<Arc<dyn Trait>>`）都被加入 `dep_type_ids` 参与拓扑排序。
2. trait 无任何实现时，其 TypeId 不在 `trait_impls` 中 → 拓扑排序直接报"TypeId 未注册"→ **启动失败**。
3. 即便拓扑侥幸通过，`inner_init` 中 Option 字段也使用 panic 版 `inject_trait_from_store`。

**后果**：
- `Option<Arc<dyn Trait>>` 在无实现时应为 `None`，实际却启动崩溃——"可选"名不副实。
- `Vec<Arc<dyn Trait>>` 在零实现时应为空 Vec（完全合法场景，如无中间件插件），实际同样启动失败。

**建议**：Option 字段改用 `try_inject_trait`（需新增）；Vec 字段无实现时返回空 Vec；拓扑排序对 optional/list 依赖在 trait 无实现时跳过而非报错。

### 2.6 DashMap 持锁回调，存在重入死锁面

**位置**：`tx-di-core/src/store.rs:97-102, 216-220`

**问题**：`inner.get(&tid)` 获取 DashMap shard 读锁**存活期间**执行 `f(self)`（factory → inner_init → 用户 init 回调）。Prototype 组件的 init 回调持有 `&Store`，若在其中调用 `store.insert_cached/insert_arc` 写同一 shard → **死锁**。当前纯读-读路径安全，但这是一个随用户代码增长随时引爆的设计隐患。

**建议**：先 clone 出 `CompRef` 再释放 guard（`drop(entry)`），然后调用 factory。

### 2.7 `#[intercept]` 中 `return` / `?` 可绕过 after 拦截器

**位置**：`tx-di-macros/src/intercept_macro.rs:86-91`

**问题**：生成的拦截代码将原方法体作为表达式内联：
```rust
let __result = #body;   // body 含 return/?
#after_block
__result
```
若 body 中含 `return xxx;` 或 `?` 运算符，会**直接从外层函数返回，跳过 after 拦截器**。after 拦截器（审计、指标、日志）静默失效。

**建议**：将 body 包进闭包/立即调用的 async 块：`let __result = (move || -> #ret_ty { #body })();`，使 `return`/`?` 只跳出内层。

### 2.8 拦截器 before 返回 Err → 转 panic

**位置**：`tx-di-macros/src/intercept_macro.rs:81-83`

**问题**：`before` 返回 `Err` 本是正常控制流（如鉴权失败），却被 `unwrap_or_else(panic!)` 转成 panic：
```rust
__chain.before_all(&__ctx).unwrap_or_else(|e| {
    panic!("[di] 拦截器拒绝 method={}: {}", ...)
});
```
对返回 `Result` 的方法应生成 `return Err(e.into())`；非 Result 方法才有理由 panic。

**建议**：当方法返回类型是 Result 时，生成 `return Err(...)`；非 Result 时保持 panic（但文档醒目标注）。

---

## 三、架构设计缺陷

### 3.1 Store 缺乏层级 / Scope 能力

**位置**：`tx-di-core/src/store.rs`

**问题**：`Store` 是全局平铺的 `DashMap<TypeId, CompRef>`，没有层级隔离（如 request-scope、session-scope）。当前只有 `Singleton` 和 `Prototype` 两种 scope，不能满足 Web 开发中常见的"请求级"依赖（如 `RequestContext`、数据库事务等）。

**建议**：增加 `Request` scope 或子 Store 机制，支持在 request 级别创建子 Store 并注入有生命周期的实例。

### 3.2 无条件注入（Conditional Injection）支持

**位置**：`tx-di-macros/src/classify/fields.rs`

**问题**：`FieldKind` 中 `Optional` 变体只支持 `Option<T>`（非 trait），而 `trait_inject_fields` 对应的 `Option<Arc<dyn Trait>>` 是 `FieldKind::TraitInject`，两者逻辑不同。但缺少 `Option<Arc<T>>`（可选普通组件注入）的支持。如果用户想实现"注入如果存在则使用，否则默认"的普通组件，目前无法做到——只能依赖 `Optional`（始终填 None）或 `Inject`（必须存在）。

**建议**：增加 `OptionalInject` 变体，支持 `Option<Arc<T>>` 的普通组件可选注入。

### 3.3 Prototype 组件的生命周期语义完全错误

**位置**：`tx-di-macros/src/codegen/meta_entry.rs:95-103` + `codegen/lifecycle.rs:56,74,98`

**问题**：
1. **shutdown 静默失效**（见前文）：`shutdown_fn` 通过 `store.try_inject::<T>()` 获取实例，Prototype 不在 store → 返回 None → shutdown 永不调用。
2. **app_init / app_async_init 操作的是即弃实例**：生命周期覆写中 `inject_from_store::<Self>` 对 Prototype 每次调用新建实例。`app_init` 初始化的实例用后即弃，用户后续注入拿到的实例从未被初始化。
3. **shutdown_fn 会新建即弃实例**：对 Prototype 调用 `try_inject` 实际会触发 factory 新建一个实例然后立刻调 shutdown——毫无意义且可能有副作用。

**建议**：
- 宏在 `scope = Prototype` + 生命周期标志（`app_init`/`app_async_init`/`app_async_run`/`shutdown`）组合时直接 `compile_error!` 拒绝
- 或给 Prototype 加实例追踪（Weak 集合），见修复计划 Fix 1.1

### 3.4 拦截器链全局表：性能瓶颈 + 内存泄漏 + ABA 风险

**位置**：`tx-di-core/src/aop.rs:209-219`

**问题**：拦截器链存储在 `static INTERCEPTOR_CHAINS: OnceLock<Mutex<HashMap<usize, Arc<InterceptorChain>>>>` 中，三重问题：

1. **性能瓶颈**：每次方法调用都要过一把全局 `Mutex` 锁，高并发下是竞争热点。
2. **内存泄漏**：key 是组件实例裸地址（`Arc::as_ptr as usize`），**从不删除**。App/组件销毁后条目永久残留，多次构建 App 的测试进程持续泄漏。
3. **ABA 风险**：旧实例释放后新分配恰好落在同一地址，会命中旧拦截链。

**建议**：改用 `DashMap<usize, Arc<InterceptorChain>>`（消除 Mutex + poison）；shutdown 时按实例地址 `remove`；或更彻底地，把链存到组件自身的隐藏字段中（`OnceLock<Arc<InterceptorChain>>`）。

### 3.6 全局注册表全量急切构建

**位置**：`tx-di-core/src/lifecycle.rs:86-115` + `COMPONENT_REGISTRY`

**问题**：
1. 链接进二进制的**所有** `#[derive(Component)]` 组件在每次 `BuildContext::new` 时全部实例化（急切单例），无 lazy、无 profile/feature 条件装配。插件只能各自发明 `enabled` 配置项，且即便 `enabled=false` 组件本体仍被构建。
2. 集成测试同一 binary 中所有测试组件互相可见、全部构建，隔离性差、拖慢启动。

**建议**：`ComponentMeta` 增加 `condition: Option<fn(&AppAllConfig) -> bool>` 或 `enabled_feature: Option<&str>`；BuildContext 支持 include/exclude 过滤器；长期支持 lazy 单例（`OnceLock` factory）。

### 3.7 Trait 注入只支持单个实现

**位置**：`tx-di-core/src/store.rs:196-242`

**问题**：`inject_trait_from_store` 只返回 `entries.first().cloned()` — 第一个匹配的实现。当有多个实现时，没有优先级 / 选择策略，且多实现时行为不确定（取决于注册顺序）。

**建议**：增加 `#[component(as_trait = dyn Trait, primary)]` 优先选择机制，或在多实现时返回错误要求显式选择。

---

## 四、宏系统 / 类型检测缺陷

### 4.1 类型检测不支持完整路径

**位置**：`tx-di-macros/src/type_utils.rs:17, 31-43, 59, 82`

**问题**：`strip_arc_type`、`extract_option_inner`、`extract_trait_from_arc` 等都只检查裸标识符（如 `Arc`、`Option`、`Vec`），不检查完整路径：
- `std::sync::Arc<T>` 不会被识别为 `Arc`
- `::std::sync::Arc<T>` 不会被识别
- `std::option::Option<T>` 不会被识别

如果用户在源码中使用完整路径或 `use std::sync::Arc as SyncArc;` 别名，字段分类会出错。

**建议**：检查路径的末段 ident 即可，不应要求只有一段路径，允许 `std::sync::Arc`、`::core::option::Option` 等形式。

### 4.2 `is_result_return_type` 使用字符串前缀匹配

**位置**：`tx-di-macros/src/intercept_macro.rs:100-115`

**问题**：`#[intercept]` 生成的代码中，判断返回值类型是否为 `Result` 使用的是字符串前缀匹配，这种方式脆弱：
- 类型别名 `type MyResult = Result<T, E>;` 不匹配
- `<T as Trait>::Result` 不匹配
- `Result <T>`（空格差异）不匹配

**建议**：使用 syn 类型结构匹配，而非字符串比较。

### 4.3 `is_arc_dyn_trait` 命名歧义

**位置**：`tx-di-macros/src/type_utils.rs:93-96`

**问题**：函数名 `is_arc_dyn_trait` 暗示检测 `Arc<dyn Trait>`，但实际检测的是 `Option<Arc<dyn Trait>>`。真正的 `Arc<dyn Trait>` 检测函数叫 `is_plain_arc_dyn_trait`。命名容易导致代码维护时误用。

**建议**：重命名为 `is_option_arc_dyn_trait`，与 `is_plain_arc_dyn_trait` 语义一致。

### 4.4 `camel_to_snake` Unicode 边缘情况

**位置**：`tx-di-macros/src/name_utils.rs:21`

**问题**：`ch.to_lowercase().next().unwrap()` 在某些 Unicode 字符上可能语义不正确（如 `İ` → `i̇` 是两个 char，`next()` 只取第一个）。虽然 DI 场景中组件名几乎不会遇到这类字符，但代码不够健壮。

**建议**：使用 `ch.to_lowercase().collect::<String>()` 而非只取第一个 char。

### 4.5 `#[tx_cst(expr)]` 在 Option/trait 字段上被静默忽略

**位置**：`tx-di-macros/src/classify/fields.rs:44-61`

**问题**：字段分类顺序为：`skip` → `Arc<dyn Trait>` → `Vec<Arc<dyn Trait>>` → `Option<Arc<dyn Trait>>` → `Option<T>` → **才轮到 `inject_expr`**。`#[tx_cst(Some(...))]` 标注在 `Option<T>` 字段、或 `#[tx_cst(...)]` 标注在任何 trait object 字段上时，表达式**被静默丢弃**，字段被填 `None` / 占位值。用户显式写的属性无效且无任何警告。

**建议**：将 `inject_expr` 检查提到类型形态判断**之前**（用户显式意图优先）；或冲突时 `compile_error!` 报错。

### 4.6 生成代码依赖用户 crate 的裸路径（宏卫生违规）

**位置**：`tx-di-macros/src/codegen/factory.rs:31,41,50,64`

**问题**：
- 配置组件生成代码写死 `serde::Deserialize`：用户 crate 必须直接依赖 serde、不得重命名、不得有同名本地模块。
- `::tracing::debug!` 同理：任何使用 `#[derive(Component)]` 的 crate 被迫依赖 tracing。

**建议**：`tx-di-core` 中 `pub use serde; pub use tracing;`，生成代码统一走 `::tx_di_core::serde::...` / `::tx_di_core::tracing::...`（对标 linkme 已在 `meta_entry.rs:73-74` 使用 `::tx_di_core::linkme`）。

### 4.7 非 Arc 普通字段静默按注入处理

**位置**：`tx-di-macros/src/classify/fields.rs:60`

**问题**：兜底分支把任意类型（如 `String`、`u32`）归为 `FieldKind::Inject`，生成 `Deps = (Arc<String>,)` 与 `deps.0.clone()` 赋给 `String` 字段 → E0308 类型不匹配。错误 span 落在宏展开处，用户难以定位原文。

**建议**：在宏中检测"字段类型既非 `Arc<..>` 又无 `#[tx_cst]`"时给出带修复建议的编译错误。

### 4.8 `Option<T>`（非 trait）字段永远为 None

**位置**：`tx-di-macros/src/codegen/component_impl.rs:46`

**问题**：build 时填 `None`，之后**永不注入**——即使 `T` 已注册。"可选依赖"实际是"永远缺席的依赖"，语义严重误导。叠加 §4.5，用户连 `#[tx_cst]` 都救不回来。

**建议**：要么实现真正的可选注入（已注册则注入、未注册则 None），要么文档明确 + 禁止该形态，或引导用 §3.2 的 `OptionalInject`。

### 4.9 其他 DX / 宏问题

| 问题 | 位置 | 建议 |
|------|------|------|
| Deps 上限 16 报"trait 未实现"晦涩错误 | `component.rs:152` | 自定义编译诊断（`#[diagnostic::on_unimplemented]`） |
| linkme 插件裁剪 footgun | 所有插件 | README + 错误信息统一给出"空 use"方案；评估 `inventory` 迁移 |
| `camel_to_snake` 缩略词反直觉 | `name_utils.rs` | `HTTPServer` → 当前 `h_t_t_p_server`，应聚合为 `http_server` |
| `as_trait` 仅支持单个 trait | `comp_attr.rs` | `as_trait: Option<Type>` → `Vec<Type>`，支持 `as_trait = (dyn A, dyn B)` |

---

## 五、生命周期 / 并发问题

### 5.1 `ins_run` 中 async_run 失败时不传播错误

**位置**：`tx-di-core/src/lifecycle.rs:333-337`

**问题**：`comp_run` 中各组件的 `async_run` 在 `tokio::spawn` 中运行，失败只打日志（`tracing::error!`），不传播错误。上层调用者无法感知组件后台任务失败。

**建议**：收集所有 task 的错误，在 shutdown 或特定时机汇总报告；或提供回调机制。

### 5.2 shutdown 顺序可能不正确

**位置**：`tx-di-core/src/lifecycle.rs:392-398`

**问题**：shutdown 使用 `metas.iter().rev()` 逆序关闭（后注册的先关闭），但"后注册"不等于"依赖反向"。正确的 shutdown 顺序应该是拓扑排序的**逆序**（被依赖者后关闭）。当前逆注册顺序可能导致先关闭被其他组件依赖的基础组件。

**建议**：使用与构建阶段相同的拓扑排序的逆序来执行 shutdown。

### 5.3 `waiting_exit` 硬编码 5 秒超时

**位置**：`tx-di-core/src/lifecycle.rs:409`

**问题**：`tokio::time::timeout(Duration::from_secs(5), handle)` 硬编码 5 秒超时，不可配置。对于有大量后台任务的复杂应用，5 秒可能不够。

**建议**：将超时时间改为可配置（通过 `#[component(init_sort)]` 或系统配置）。

### 5.4 BuildContext 的 Store 无法在 build 后访问

**位置**：`tx-di-core/src/lifecycle.rs:234-244`

**问题**：`build()` 方法使用 `std::mem::replace(&mut self.store, Store::new())` 将 Store 移出，之后 `self` 被部分移动（`metas` 也被 `take`），调用者无法在 `build()` 之后继续使用 `BuildContext`。这是有意设计，但如果用户想在 build 之前注入某个组件并在 build 后对比验证，就很不方便。

---

## 六、错误处理 / 可观测性

### 6.1 错误信息缺少 TypeId 可读化

**位置**：`tx-di-core/src/store.rs:97-127`

**问题**：错误消息中只显示 `type_name` 和 `TypeId {:?}`，如果组件是泛型相关的，`type_name` 会非常冗长且难以阅读（Rust 的 `type_name` 包含完整模块路径和泛型参数）。

**建议**：在 `ComponentMeta` 中存储简化后的组件名（已有 `name` 字段），并在错误消息中引用。

### 6.2 缺少运行时组件健康检查

**问题**：当前无任何组件健康检查机制。基础组件（数据库连接池、缓存连接）失效时，上层组件无法感知。应用层只能在使用失败时才发现。

**建议**：增加 `fn health_check(&self) -> RIE<()>` 生命周期钩子，支持主动探测。

### 6.3 `DiErr` 错误码未全部使用

**位置**：`tx-di-core/src/error.rs:15-26`

**问题**：定义了 4 个错误码（`RegistryError`、`AsyncInitError`、`TaskPanic`、`InjectError`），但 `AsyncInitError` 和 `TaskPanic` 在代码中从未被实际使用。这容易让维护者误以为某些错误被正确处理了。

**建议**：清理未使用的错误码，或在相应位置实际使用它们。

### 6.4 shutdown 缺陷集合

| 问题 | 位置 | 说明 |
|------|------|------|
| **可能双重执行** | `lifecycle.rs:377-380, 423` | `ins_run` 错误路径调 `app_clone.shutdown()`，`waiting_exit` 又调一次，无幂等保护 |
| **只支持同步** | `meta_entry.rs:62` | `shutdown_fn: fn(&Store)`，需要异步清理（flush、关闭连接）的组件只能 `block_on` |
| 硬编码超时 | `lifecycle.rs:409,426` | 后台 join 5s、退出前 `sleep(200ms)` 均不可配置 |

**建议**：`AtomicBool` 幂等门闩；增加 `async_shutdown_fn` 或统一为 `BoxFuture`；超时可配置。

### 6.5 配置系统缺陷

**位置**：`tx-di-core/src/config.rs`

**问题**：
1. **错误策略自相矛盾**：`AppAllConfig::get()` 用 `.ok()` **静默吞掉**反序列化错误——配置写错类型时 `get_or_default` 悄悄回退默认值；而配置组件路径解析失败却 panic。同一份错误配置，两条路径两种行为。
2. **全局静态污染**：`CONFIG_PATH` 经 `set_sys_config` 写入全局静态，同进程多 App（如并行测试）互相覆盖。
3. **仅支持 TOML**：无环境变量覆盖、无多文件叠加（base + env）、无热更新。

**建议**：统一严格模式（默认 panic 或返回 Err），宽松模式通过 `get_or_default` 显式选择；CONFIG_PATH 改为 Store/App 实例字段；长期支持多层配置源。

### 6.6 文档漂移

| 位置 | 问题 |
|------|------|
| `store.rs:92` | 注释写"组件未注册时 panic"，实际 `inject` 返回 `Result` |
| `aop.rs:13` | 模块示例中若为单元结构体 derive 不支持（仅支持具名字段），示例不可编译 |
| 宏 crate README | 回调示例签名需逐一与实际生成的代码要求的**模块级自由函数**核对 |

---

## 七、性能问题

### 7.1 `auto_register_all` 中 O(n²) 线性查找

**位置**：`tx-di-core/src/lifecycle.rs:110`

**问题**：`metas.iter().find(|m| (m.type_id)() == *tid)` 在循环中对每个 `tid` 做线性查找，时间复杂度为 O(n²)。当组件数量较大时（>100），可能有性能问题。

**建议**：预先构建 `HashMap<TypeId, &ComponentMeta>`，将查找降为 O(1)。

### 7.2 `TraitImplEntry::concrete_tid` 存储函数指针

**位置**：`tx-di-core/src/store.rs:187`

**问题**：每次 trait 注入时都要调用 `(entry.concrete_tid)()` 函数指针来获取 TypeId，而 TypeId 是编译期常量，应该在编译时存入。

**建议**：将 `concrete_tid` 改为直接存储 `TypeId` 而非函数指针，或使用 `LazyLock<TypeId>`。

### 7.3 `AppAllConfig::get` 每次 clone TOML 值

**位置**：`tx-di-core/src/config.rs:89-92`

**问题**：`self.get_value(key)?; T::deserialize(value.clone()).ok()` 每次获取配置都 clone 整个 TOML 值进行反序列化。对于大型配置文件和多次读取的场景，clone 开销不可忽视。虽然配置读取通常只在启动阶段发生，但如果配置组件使用 Prototype 作用域，每次构造都会重新 clone + 反序列化。

**建议**：对配置组件使用缓存，Singleton 组件在 build 阶段反序列化一次即可。

### 7.4 `InterceptorChain::push` 总是分配新 Arc

**位置**：`tx-di-core/src/aop.rs`

**问题**：`self.interceptors.push(Arc::new(interceptor))` 中 `push` 方法消耗所有权，但参数类型是 `I: Interceptor`（值类型），无法复用已有的 `Arc<I>`。这导致每次 push 都额外分配。

**建议**：增加 `push_arc(&mut self, interceptor: Arc<dyn Interceptor>)` 方法，允许直接传入已 Arc 包装的拦截器。

### 7.5 async_init 严格串行

**位置**：`tx-di-core/src/lifecycle.rs:314-317`

**问题**：`async_init` 对所有组件严格串行 await，无依赖关系的组件本可按拓扑分层并行执行，大量 IO 型初始化（DB 连接、注册中心注册）时启动时间线性叠加。

**建议**：对拓扑排序结果分层，同层内组件并发执行 `async_init`（`join_all`），层间串行。

### 7.6 comp_run 对空实现也 spawn

**位置**：`tx-di-core/src/lifecycle.rs:327-336`

**问题**：`comp_run` 对**每个**组件（含未覆写 `async_run` 的空实现）都 `tokio::spawn`，N 个组件 N 个空任务，浪费 tokio 资源。

**建议**：`ComponentMeta` 加 `has_async_run: bool` 字段，无实现的组件直接跳过。

---

## 八、代码质量问题

### 8.1 死代码

| 位置 | 标记 |
|------|------|
| `tx-di-macros/src/type_utils.rs:132-136` (`strip_arc_tokens`) | `#[allow(dead_code)]` |
| `tx-di-macros/src/attr/comp_attr.rs:57-64` (`has_any_lifecycle`) | `#[allow(dead_code)]` |

**建议**：清理或恢复使用。

### 8.2 注释掉的依赖

**位置**：`tx-di-core/Cargo.toml`

**问题**：`thiserror`、`serde_json`、`chrono`、`tracing-subscriber` 等被注释掉的依赖，以及 `tx-di-core` 在 `tx-di-macros/Cargo.toml` 中被注释掉的 dev-dependency。

**建议**：清理未使用的依赖声明。

### 8.3 API 污染

**位置**：`tx-di-core/src/lib.rs`

**问题**：`pub use dashmap;` 和 `pub use toml;` 将整个外部 crate 暴露给下游，下游用户可能意外依赖这些 re-export 的类型。如果未来升级或替换这些依赖，将造成 breaking change。

**建议**：只 re-export 必要的类型（`DashMap`、`Value`、`map`），不要整 crate 暴露。

### 8.4 `unwrap()` vs `expect()` 不一致

**位置**：`tx-di-macros/src/codegen/component_impl.rs:53`

**问题**：同样逻辑保证的场景下，有的用 `unwrap()`，有的用 `expect()`，缺少一致性。`idx` 查找用了 `unwrap()` 而没用 `expect("按名称查找索引不应失败")`。

---

## 九、功能缺口

### 9.1 缺少以下功能性支持

| 功能 | 重要程度 | 说明 |
|------|----------|------|
| **条件注入** | 高 | 无法基于条件（如 feature flag、配置）决定是否注册组件 |
| **Provider / Factory 自定义** | 中 | 无法在运行时动态选择实现（如根据配置选择不同的数据库驱动） |
| **装饰器模式** | 中 | 当前 AOP 只有 `before` / `after`，不支持 `around`（完全替换调用） |
| **多模块 / 子容器** | 中 | 无法在同一个 App 中创建隔离的组件子集 |
| **循环依赖破环** | 中 | 拓扑直接报错，不支持 `Weak<T>` / `Lazy<T>` 打破环 |
| **并行初始化** | 中 | async_init 串行，无法拓扑分层并发 |
| **异步 factory** | 低 | build 同步，异步资源（如连接池）只能挪到 async_init |
| **可观测性** | 低 | debug_registry 只打印，缺少健康检查钩子、组件状态内省 API |
| **循环依赖检测增强** | 低 | 循环依赖检测只返回参与循环的组件名，无法指出具体哪条边造成循环 |
| **Lazy 注入** | 低 | 不支持延迟初始化（只在首次访问时才创建组件） |
| **组件 override（测试替身）** | 低 | 测试环境中无法用 mock/stub 替换真实组件 |
| **多实现限定符** | 低 | 只取 first()，不支持 named/qualifier

### 9.2 测试覆盖不足

**位置**：`tx-di-core/tests/test_component.rs`

**问题**：有 42 个测试，覆盖了基本场景，但缺失：
- `TraitInjectRequired`（`Arc<dyn Trait>` 必选注入）的端到端测试
- Prototype 组件 shutdown 不被调用的测试（验证 bug 存在）
- 并发 trait 注入的 race condition 测试
- `#[component(init)]` (inner_init) 回调的测试
- 配置错误路径（损坏的 TOML 文件）测试
- 依赖不存在时的错误消息格式测试

---

## 十、优先级建议

| 优先级 | 缺陷 | 影响 |
|--------|------|------|
| **P0** | §1.1 unsafe 零值 Arc 的 UB 风险 | 运行时 UB，可能 crash |
| **P0** | §1.2 泛型结构体不可用 | 功能完全不可用 |
| **P1** | §2.5 "可选" trait 注入不可选 | 启动崩溃 |
| **P1** | §2.7 `#[intercept]` return/? 绕过 after | 审计/指标静默失效 |
| **P1** | §2.8 拦截器 before Err→panic | 鉴权失败变崩溃 |
| **P1** | §3.3 Prototype 生命周期语义完全错误 | 资源泄漏 + 初始化失效 |
| **P1** | §4.5 `#[tx_cst]` 被静默忽略 | 数据错误，无警告 |
| **P1** | §5.2 shutdown 顺序不正确 | 关闭时可能 crash |
| **P1** | §2.2 拓扑排序错误转 panic | 生产环境崩溃 |
| **P2** | §2.6 DashMap 重入死锁面 | 偶发死锁 |
| **P2** | §3.4 拦截器链全局表（锁+泄漏+ABA） | 高 QPS + 内存 + 安全性 |
| **P2** | §4.1 类型检测不支持完整路径 | 正确性问题 |
| **P2** | §4.6 宏卫生裸路径依赖 | 编译错误不可理解 |
| **P2** | §4.7 非 Arc 字段静默误判 | 编译错误 span 不可定位 |
| **P2** | §6.5 配置系统静默吞错 | 行为不一致 |
| **P2** | §3.1 缺少 Request scope | 功能缺失 |
| **P2** | §3.6 全局注册表急切构建 | 启动慢 + 测试隔离差 |
| **P3** | §7.1 O(n²) 线性查找 | 组件数 >100 时性能 |
| **P3** | §7.5 async_init 串行 | 启动慢 |
| **P3** | §7.6 comp_run 对空实现 spawn | 浪费 tokio 资源 |
| **P3** | §6.4 shutdown 双重执行 + 仅同步 | 不可靠 |
| **P3** | §6.6 文档漂移 | 误导使用者 |
| **P3** | §8.3 API 污染 | 可维护性 |
| **P3** | 其他 | 改善体验 |

---

# 附录：详细修复计划

> 按优先级分批次执行，每批可独立交付。括号内为预估工作量（含测试）。

---

## 第 0 批：紧急修复（P0，预计 1-2 天）

### Fix 0.1 — 消除 `unsafe` 零值 Arc 的 UB 风险 【§1.1】

**目标**：移除 `mem::zeroed()` + `ptr::write` 的 unsafe 模式，改为在 build 阶段直接注入。

**当前问题链路**：
```
component_impl.rs::build_fields → TraitInjectRequired → zeroed()
inner_init.rs            → TraitInjectRequired → ptr::write() 覆盖
```

如果 panic 发生在 ptr::write 之前，drop 零值 Arc → UB。

**修复方案**：

1. **`tx-di-macros/src/codegen/component_impl.rs`**（`build_fields` 函数）：
   - 移除 `TraitInjectRequired` 的 `unsafe { zeroed() }` 分支
   - 改为：在 build 阶段接收 `store` 参数，直接调用 `inject_trait_from_store` 获取真实值
   - 修改 `factory` 模块，将 `store` 传入 `build` 调用

2. **`tx-di-macros/src/codegen/factory.rs`**：
   - 当前工厂签名：`|store| { let deps = resolve(store); Self::build(deps) }`
   - 改为：`|store| { let deps = resolve(store); Self::build(deps, store) }` — 将 store 也传入 build

3. **`tx-di-macros/src/codegen/inner_init.rs`**：
   - 移除 `TraitInjectRequired` 的 `ptr::write` 分支
   - `inner_init` 不再处理 TraitInjectRequired（已在 build 中完成）

4. **`tx-di-core/src/component.rs`**（`Component` trait）：
   - `fn build(deps: Self::Deps) -> Self;` 改为 `fn build(deps: Self::Deps, store: &Store) -> Self;`
   - 或提供一个新的关联函数 `fn build_with(deps: Self::Deps, store: &Store) -> Self` 作为宏生成的调用目标

5. **测试**：新增 `TraitInjectRequired` 端到端测试，验证正常注入和注入失败两条路径

**影响范围**：
- `tx-di-macros/src/codegen/component_impl.rs` — 修改 build_fields
- `tx-di-macros/src/codegen/factory.rs` — 修改工厂闭包
- `tx-di-macros/src/codegen/inner_init.rs` — 删除 unsafe 分支
- `tx-di-core/src/component.rs` — 可能需要修改 Component trait
- `tx-di-core/tests/test_component.rs` — 新增测试
- 所有手动实现 `Component` trait 的代码需同步修改签名

**验证标准**：
- `cargo test -p tx-di-core` 全部通过
- 新增的 TraitInjectRequired 端到端测试通过
- `cargo clippy -p tx-di-core -p tx-di-macros` 无 unsafe 相关警告
- MIRI 跑 Regression 测试验证无 UB

---

### Fix 0.2 — 泛型结构体明确不支持 【§1.2】

**目标**：将死胡同 API 改为明确的错误提示。

**修复方案**：

1. **`tx-di-macros/src/codegen/mod.rs:66-71`**：
   - 将错误消息改为更明确的：
     ```
     "不支持泛型结构体。请考虑以下替代方案：
      1. 使用 newtype 包装具体类型
      2. 在 impl 块中为每个具体类型手动实现 Component"
     ```

2. **`tx-di-macros/src/attr/comp_attr.rs:220-224`**：
   - 移除 `for` 参数的解析逻辑（不再假装支持）
   - 或保留解析但生成一个编译时错误

3. **`tx-di-core/tests/test_component.rs`**：
   - 新增 `#[test]` 验证泛型结构体使用 `#[derive(Component)]` 时报错消息友好且有用

**验证标准**：
- `cargo test -p tx-di-core` 全部通过
- 泛型结构体编译错误消息清晰，建议了替代方案

---

## 第 1 批：稳定性修复（P1，预计 3-5 天）

### Fix 1.1 — Prototype shutdown 支持 【§3.3】

**目标**：让 Prototype 组件的 `#[component(shutdown)]` 回调切实被调用。

**当前问题链路**：
```
meta_entry.rs:95 → store.try_inject::<T>() → Prototype 不在 store → None → shutdown 跳过
```

**修复方案**：

1. **`tx-di-core/src/store.rs`**（Store 结构体）：
   - 新增字段 `prototype_instances: DashMap<TypeId, Vec<Weak<dyn Any + Send + Sync>>>`
   - 在 `CompRef::Factory` 闭包返回时将强引用 `Arc` 转为 `Weak` 存入 `prototype_instances`

2. **`tx-di-core/src/store.rs`**（Store impl）：
   - 新增方法 `pub fn shutdown_prototypes(&self)`：
     - 遍历 `prototype_instances`
     - 对每个 `Weak` 调用 `upgrade()`，如果存活则调用 `shutdown()`
     - 清理已释放的 Weak 条目

3. **`tx-di-macros/src/codegen/meta_entry.rs:95-103`**：
   - shutdown_fn 改为：先 `try_inject`，失败则从 `store.prototype_instances` 获取并尝试升级 Weak

4. **`tx-di-core/src/lifecycle.rs:392-398`**（App::shutdown）：
   - 在遍历 metas 之后额外调用 `self.store.shutdown_prototypes()`

5. **`configs/` 示例配置文件**：
   - 新增配置项 `prototype_shutdown_timeout_ms` 控制 Weak 升级超时

**测试**：
- 新增 test：Prototype 组件 + `#[component(shutdown)]`，验证 shutdown 回调被调用
- 新增 test：Prototype 组件全部 drop 后 shutdown，不应 panic

---

### Fix 1.2 — shutdown 顺序修正为拓扑逆序 【§5.2】

**目标**：shutdown 按照构建时的拓扑顺序**严格逆序**执行，确保被依赖者不会先于依赖者关闭。

**当前代码**：`lifecycle.rs:392-398` 用 `metas.iter().rev()`（逆注册顺序），不等于逆拓扑顺序。

**修复方案**：

1. **`tx-di-core/src/lifecycle.rs`**（`App` 结构体）：
   - 新增字段 `sorted_metas: Vec<&'static ComponentMeta>`（按拓扑顺序存储）

2. **`tx-di-core/src/lifecycle.rs`**（`BuildContext::build`）：
   - 将已排序的 `self.metas` 存入 `App.sorted_metas`

3. **`tx-di-core/src/lifecycle.rs`**（`App::init` / `App::async_init`）：
   - 改用 `sorted_metas` 保证顺序一致

4. **`tx-di-core/src/lifecycle.rs`**（`App::shutdown`）：
   - 改为 `self.sorted_metas.iter().rev()` — 拓扑逆序

**测试**：
- 新增 test：A 依赖 B，B 的 shutdown 计数器晚于 A 的 shutdown 计数器递增

---

### Fix 1.3 — 拓扑排序错误传播（不转 panic）【§2.2】

**目标**：将 `auto_register_all` 中的 `.unwrap_or_else(panic!)` 改为错误传播。

**修复方案**：

1. **`tx-di-core/src/lifecycle.rs:86-115`**：
   - `auto_register_all` 返回值从 `()` 改为 `RIE<()>`
   - 将 `topo_sort(...).unwrap_or_else(|e| panic!("{}", e))` 改为 `topo_sort(...)?`

2. **`tx-di-core/src/lifecycle.rs:69-83`**（`BuildContext::new`）：
   - 由于 `auto_register_all` 现在返回 `Result`，需要处理错误
   - **方案 A**：`new` 也返回 `RIE<Self>`（breaking change）
   - **方案 B**：保持 `new` 的签名，内部 `auto_register_all().expect("...")` 并在 new 上标注 `# Panics`
   - **推荐方案 A**，统一错误处理

**影响范围**：
- 所有调用 `BuildContext::new()` 的地方需要改为 `BuildContext::new(...)?` 或 `.unwrap()`
- 示例代码和测试代码需同步更新

---

### Fix 1.4 — 消除 config / factory 中的 panic 路径 【§2.3, §2.4】

**目标**：将配置加载和工厂中的 `panic!` 全部转为 `Result` 错误。

**修复方案**：

1. **`tx-di-core/src/config.rs`**（`AppAllConfig::new` / `load_config`）：
   - 返回类型从 `Self` / `Value` 改为 `RIE<Self>` / `RIE<Value>`
   - 所有 `unwrap_or_else(panic!)` → `map_err(|e| AppError::with_context(DiErr::ConfigError, ...))?`
   - 新增错误码 `DiErr::ConfigError`

2. **`tx-di-macros/src/codegen/factory.rs:32-33,42,59,61-62`**：
   - 配置反序列化失败、Deps::resolve 失败、inner_init 失败
   - 改为：在 factory 闭包中返回 `Result<Arc<T>, AppError>` 而非直接 panic
   - 需要修改 `CompRef::Factory` 的闭包签名：`Fn(&Store) -> Result<Arc<dyn Any + Send + Sync>, AppError>`

3. **`tx-di-core/src/store.rs`**（`CompRef` 枚举）：
   - `Factory` 变体闭包返回类型改为 `Result<Arc<dyn Any + Send + Sync>, AppError>`
   - `inject` 方法中 `CompRef::Factory(f) => f(self)` 改为 `CompRef::Factory(f) => f(self)?`

4. **`tx-di-core/src/store.rs:221-226, 233-241`**（`inject_trait_from_store`）：
   - 改为返回 `Result<Arc<T>, AppError>` 而非 panic

5. **`tx-di-core/src/error.rs`**：
   - 新增 `ConfigError` 和 `TraitInjectError` 错误码

**影响范围**：
- CompRef::Factory 闭包签名变更 → 所有注册工厂的地方
- BuildContext::register_factory → 需处理 Result
- Store::inject → 需处理 Result

---

### Fix 1.5 — 修复"可选" trait 注入 【§2.5】

**目标**：`Option<Arc<dyn Trait>>` 无实现时返回 None（不崩溃），`Vec<Arc<dyn Trait>>` 无实现时返回空 Vec。

**修复方案**：

1. **`tx-di-core/src/store.rs`**：
   - 新增 `try_inject_trait_from_store<T>(store) -> Option<Arc<T>>`，不 panic 版
   - 新增 `inject_available_traits_from_store<T>(store) -> Vec<Arc<T>>`，注入所有已注册实现（不存在时返回空 Vec）

2. **`tx-di-macros/src/codegen/inner_init.rs`**：
   - `TraitInject`（Option 字段）改用 `try_inject_trait_from_store`
   - `TraitInjectList`（Vec 字段）改用 `inject_available_traits_from_store`

3. **`tx-di-macros/src/codegen/meta_entry.rs`**：
   - `trait_inject_fields` 和 `list_trait_fields` **不加入** dep_type_ids（不构成硬依赖）

4. **`tx-di-core/src/topology.rs`**：
   - 拓扑排序对 trait 依赖未找到实现时，跳过（而非报错）

### Fix 1.6 — 修复 `#[intercept]` return/? 绕过 after + before Err→panic 【§2.7, §2.8】

**目标**：body 内 `return`/`?` 不跳过 after；before Err 返回 Result 而非 panic。

**修复方案**：

1. **`tx-di-macros/src/intercept_macro.rs`**：
   - sync 方法：body 包进 `|| { #body }()` 立即调用闭包
   - async 方法：body 包进 `async move { #body }.await`
   - Result 返回类型：before Err → `return Err(e.into())`
   - 非 Result 返回类型：before Err → `panic!`（标注文档）或 `compile_error!`

**验证标准**：
- 新增 test：方法中 `return` 后 after 仍被调用
- 新增 test：before 返回 Err 时，Result 方法拿到 Err

---

### Fix 1.7 — 修复 `#[tx_cst]` 被静默忽略 【§4.5】

**目标**：`#[tx_cst(expr)]` 优先级高于类型推断，或冲突时明确报错。

**修复方案**：

**`tx-di-macros/src/classify/fields.rs:44-61`**（`classify_fields`）：
- 将 `inject_expr` 检查提前到 `is_plain_arc_dyn_trait` 之前
- 如果 `#[tx_cst(expr)]` 与 trait/Arc 类型同时存在，emit `compile_error!`

---

## 第 2 批：架构增强（P2，预计 5-7 天）

### Fix 2.1 — 类型检测支持完整路径 【§4.1】

**目标**：让 `type_utils` 能识别 `std::sync::Arc<T>`、`::core::option::Option<T>` 等形式。

**修复方案**：

1. **`tx-di-macros/src/type_utils.rs`**：
   - 新增辅助函数 `fn is_type_ident(ty: &Type, name: &str) -> bool`：
     ```rust
     fn is_type_ident(ty: &Type, name: &str) -> bool {
         if let Type::Path(tp) = ty {
             if let Some(seg) = tp.path.segments.last() {
                 return seg.ident == name;
             }
         }
         false
     }
     ```
   - 替换 `strip_arc_type`、`extract_option_inner`、`extract_trait_from_arc`、`extract_trait_from_vec_arc` 中裸 ident 比较为 `is_type_ident` 调用
   - 保留 `segs.len() == 1` 的检查（不引入新的歧义风险），但改为只检查末段 ident

2. **`tx-di-macros/src/type_utils.rs`**（`extract_trait_from_arc` 等）：
   - 允许 `segs.len() >= 1`（而非 `== 1`）
   - 只要求末段 ident 是 `Arc`/`Option`/`Vec`

**注意**：不支持 type alias（如 `type MyArc<T> = Arc<T>`），这需要类型解析，超出了 proc-macro 的能力范围。应在文档中说明限制。

**测试**：
- 新增测试组件使用 `std::sync::Arc<DbPool>` 作为字段类型，验证注入正常

---

### Fix 2.2 — AOP 拦截器链改用 `DashMap` 【§3.4】

**目标**：消除全局 `Mutex<HashMap>` 的性能瓶颈。

**修复方案**：

1. **`tx-di-core/src/aop.rs`**：
   - 将 `static INTERCEPTOR_CHAINS: OnceLock<Mutex<HashMap<usize, Arc<InterceptorChain>>>>`
   - 改为 `static INTERCEPTOR_CHAINS: OnceLock<DashMap<usize, Arc<InterceptorChain>>>`
   - `set_interceptor_chain`：`chains_map().insert(key, chain)`
   - `get_interceptor_chain`：`chains_map().get(&key).map(|r| r.clone())`
   - 移除 `.lock().unwrap()` 调用，不再有 poison 风险

2. **额外收益**：此改动同时修复 §2.1（Mutex Poisoning）

3. **注意**：`DashMap` 已引入为依赖（store.rs 中在用），无额外依赖成本

---

### Fix 2.3 — 新增 `OptionalInject` 字段类型 【§3.2】

**目标**：支持 `Option<Arc<T>>` 字段 — "组件注册了则注入，未注册则为 None"。

**修复方案**：

1. **`tx-di-macros/src/classify/fields.rs`**（`FieldKind` 枚举）：
   - 新增变体 `OptionalInject { ty: Type }` — `Option<Arc<T>>` 形式

2. **`tx-di-macros/src/classify/fields.rs`**（`classify_fields` 函数）：
   - 在 `is_option_type` 之后、`Inject` 之前插入：
     ```rust
     } else if is_option_arc_type(&field.ty) {
         // Option<Arc<T>> 而非 Option<Arc<dyn Trait>>
         let inner = extract_inner_from_option_arc(&field.ty);
         if !is_trait_object(&inner) {
             FieldKind::OptionalInject { ty: field.ty.clone() }
         } else {
             // Option<Arc<dyn Trait>> → TraitInject（已有逻辑）
             FieldKind::TraitInject { ty: field.ty.clone() }
         }
     ```
   - 在 `type_utils.rs` 中新增 `is_option_arc_type` 和 `extract_inner_from_option_arc`

3. **`tx-di-macros/src/codegen/component_impl.rs`**（`build_fields`）：
   - `OptionalInject` 分支：`#fname: store.try_inject::<#inner_ty>(),`

4. **`tx-di-macros/src/codegen/meta_entry.rs`**（`dep_type_ids`）：
   - `OptionalInject` **不加入** dep_type_ids（不构成硬依赖）

**测试**：
- 新增 test：`Option<Arc<RegisteredComponent>>` 正常注入
- 新增 test：`Option<Arc<UnregisteredComponent>>` 返回 None，不启动失败

---

### Fix 2.4 — `InterceptorChain::push_arc` 【§7.4】

**目标**：允许直接传入已 Arc 包装的拦截器，避免额外分配。

**修复方案**：

1. **`tx-di-core/src/aop.rs`**（`InterceptorChain` impl）：
   ```rust
   pub fn push_arc(&mut self, interceptor: Arc<dyn Interceptor>) {
       self.interceptors.push(interceptor);
   }
   ```
2. **`tx-di-macros/src/codegen/intercept.rs`**：
   - 在拦截器链初始化代码中，如果拦截器本身是 DI 组件，使用 `push_arc`（已有 Arc），否则使用 `push`

---

### Fix 2.5 — 拦截器链泄漏修复与 AOP 增强 【§2.1】

**目标**：修复全局 HashMap 永不清理的泄漏 + 增加 around 拦截器。

**修复方案**：

1. **`tx-di-core/src/aop.rs`**：
   - 新增 `pub fn remove_interceptor_chain(key: usize)` 方法
   - 在 `meta_entry.rs` 的 shutdown_fn 中，对于有 interceptors 的组件，调用 `remove_interceptor_chain`
   - 注：改用 `DashMap` 后无锁性能问题，key space 是 usize，实际组件数有限，泄漏影响较小

2. **around 拦截器（§9.1）**：
   - `Interceptor` trait 新增默认方法：
     ```rust
     fn around(&self, _ctx: &CallContext, proceed: Box<dyn FnOnce() -> CallResult + Send>) -> CallResult {
         proceed()
     }
     ```
   - `InterceptorChain` 新增 `around_all` 方法，构建洋葱链调用 `around`

---

### Fix 2.6 — 消除 DashMap 重入死锁面 【§2.6】

**目标**：Store 持 shard 锁期间不再回调用户代码。

**修复方案**：

**`tx-di-core/src/store.rs:97-102`**：
```rust
// 旧：持锁回调
match self.inner.get(&tid) {
    Some(entry) => match &*entry {
        CompRef::Factory(f) => f(self),  // factory 可能回调 Store
        ...
    }
}

// 新：先 clone 再释放 guard
let entry = self.inner.get(&tid).map(|e| e.clone());
drop::<Option<_>>(entry_guard);
match entry {
    Some(CompRef::Factory(f)) => f(self),
    ...
}
```

### Fix 2.7 — 宏卫生：serde/tracing 经 tx-di-core 导入 【§4.6】

**修复方案**：

1. **`tx-di-core/src/lib.rs`**：`pub use serde; pub use tracing;`
2. **`tx-di-macros/src/codegen/factory.rs`**：
   - `serde::Deserialize` → `::tx_di_core::serde::Deserialize`
   - `::tracing::debug!` → `::tx_di_core::tracing::debug!`
3. **其他生成代码**：同理替换所有裸路径依赖

### Fix 2.8 — 非 Arc 字段编译期报错 【§4.7】

**修复方案**：

**`tx-di-macros/src/classify/fields.rs:59-60`**：
- 兜底 `FieldKind::Inject` 之前，检测字段类型是否为 `Arc<...>` 形式
- 如果不是 → `return Err(syn::Error::new_spanned(ty, "非 Arc 字段请使用 #[tx_cst(expr)] 或 #[tx_cst(skip)]"))`

### Fix 2.9 — 配置系统统一错误策略 + CONFIG_PATH 实例化 【§6.5】

**修复方案**：

1. **`tx-di-core/src/config.rs`**：`get()` 改为 `get_or_err()` (返回 `RIE<T>`)，`get_or_default` 保留宽松版
2. **`tx-di-core/src/config.rs:49-52`**：移除 `set_sys_config(CONFIG_PATH, ...)` 全局静态写入；`config_path` 存入 `AppAllConfig` 实例字段
3. **`tx-di-core/src/config.rs:89-92`**：`get()` 的 `.ok()` 改为 `?`（默认严格，可选宽松）

### Fix 2.10 — 全局注册表条件装配 【§3.6】

**修复方案**：

1. **`tx-di-core/src/registry.rs`**（`ComponentMeta`）：新增 `condition: Option<fn(&AppAllConfig) -> bool>`
2. **`tx-di-macros/src/attr/comp_attr.rs`**：支持 `#[component(condition = "cfg_key")]`
3. **`tx-di-core/src/lifecycle.rs`**（`auto_register_all`）：跳过 condition 返回 false 的组件

---

## 第 3 批：质量提升（P3，预计 3-4 天）

### Fix 3.1 — O(n²) 线性查找优化 【§7.1】

**修复方案**：

**`tx-di-core/src/lifecycle.rs:108-113`**：
```rust
// 旧：O(n²)
for tid in &sorted_ids {
    if let Some(meta) = metas.iter().find(|m| (m.type_id)() == *tid) {
        ...
    }
}

// 新：O(n)
let meta_map: HashMap<TypeId, &ComponentMeta> = metas
    .iter()
    .map(|m| ((m.type_id)(), *m))
    .collect();
for tid in &sorted_ids {
    if let Some(meta) = meta_map.get(tid) {
        ...
    }
}
```

---

### Fix 3.2 — API 去污染 【§8.3】

**修复方案**：

**`tx-di-core/src/lib.rs`**：
```rust
// 旧（污染下游命名空间）
pub use dashmap;
pub use dashmap::DashMap;
pub use toml;
pub use toml::Value;
pub use toml::map;

// 新（只导出必要类型）
pub use dashmap::DashMap;
pub use toml::Value;
pub use toml::map;
// 移除 pub use dashmap; pub use toml;
```

**影响范围**：检查所有 `tx_di_core::dashmap::*` 和 `tx_di_core::toml::*` 的引用方，改为直接依赖对应 crate。

---

### Fix 3.3 — 等待退出超时可配置 【§5.3】

**修复方案**：

1. **`tx-di-core/src/lifecycle.rs`**（`App` 结构体）：
   - 新增字段 `shutdown_timeout: Duration`

2. **`tx-di-core/src/lifecycle.rs`**（`BuildContext::build`）：
   - 新增参数或从 `AppAllConfig` 读取
   - 配置 key: `system.shutdown_timeout_secs`，默认 5

3. **`tx-di-core/src/lifecycle.rs:409`**：
   - `Duration::from_secs(5)` → `self.shutdown_timeout`

---

### Fix 3.4 — 错误消息增强 【§6.1】

**修复方案**：

1. **`tx-di-core/src/store.rs:114-127`**（注入失败）：
   - 错误消息中加入已注册组件名列表（从 `COMPONENT_REGISTRY` 读取），方便排查

2. **`tx-di-core/src/store.rs:221-226, 233-241`**（trait 注入失败）：
   - 列出当前注册的该 trait 的所有实现

---

### Fix 3.5 — 死代码清理 【§8.1】

1. 移除或恢复 `strip_arc_tokens`（评估是否有用）
2. 移除 `has_any_lifecycle` 或用于编译期优化
3. 清理 Cargo.toml 中注释掉的依赖
4. 清理或使用 `DiErr::AsyncInitError` / `TaskPanic`

---

### Fix 3.6 — unwrap/expect 统一 【§8.4】

将 `component_impl.rs:53` 的 `unwrap()` 改为 `expect("...")`，与其他同类代码保持一致。

---

### Fix 3.7 — `is_arc_dyn_trait` 重命名 【§4.3】

```rust
// 旧
pub fn is_arc_dyn_trait(ty: &Type) -> bool { ... }        // 实际检测 Option<Arc<dyn Trait>>
pub fn is_plain_arc_dyn_trait(ty: &Type) -> bool { ... }  // 实际检测 Arc<dyn Trait>

// 新
pub fn is_option_arc_dyn_trait(ty: &Type) -> bool { ... }
pub fn is_plain_arc_dyn_trait(ty: &Type) -> bool { ... }
```

所有调用 `is_arc_dyn_trait` 的地方同步更新（`classify/fields.rs`、`codegen/mod.rs`）。

---

### Fix 3.8 — `camel_to_snake` Unicode 修复 【§4.4】

```rust
// 旧
ch.to_lowercase().next().unwrap()

// 新
ch.to_lowercase().to_string()
```

---

### Fix 3.9 — shutdown 幂等化 + 异步 shutdown 支持 【§6.4】

**修复方案**：

1. **`tx-di-core/src/lifecycle.rs`**（`App`）：新增 `shutdown_called: AtomicBool` 门闩
2. **`tx-di-core/src/lifecycle.rs`**（`App::shutdown`）：`if shutdown_called.swap(true) { return; }`
3. **`tx-di-core/src/registry.rs`**（`ComponentMeta`）：新增 `async_shutdown_fn: Option<fn(...) -> BoxFuture<()>>`（长期）

### Fix 3.10 — async_init 分层并行 · comp_run 跳过空实现 【§7.5, §7.6】

**修复方案**：

1. **`tx-di-core/src/lifecycle.rs`**：
   - `async_init`：对拓扑排序结果按依赖深度分层，同层组件 `join_all` 并发，层间 await
2. **`tx-di-macros/src/codegen/meta_entry.rs`**：
   - 新增 `has_async_run: bool`（有 `app_async_run` 标志时为 true）
3. **`tx-di-core/src/lifecycle.rs`**（`comp_run`）：
   - 跳过 `has_async_run == false` 的组件

### Fix 3.11 — Deps 上限 16 自定义诊断 【§4.9】

**修复方案**：

**`tx-di-core/src/component.rs`**：
- 对 >16 元组使用 `#[diagnostic::on_unimplemented(message = "Deps 元组最多支持 16 个依赖组件")]`

---

## 第 4 批：测试补全（预计 1-2 天）

### Test 4.1 — TraitInjectRequired 端到端测试

```
定义 trait MyRepo: Any + Send + Sync
定义组件 ImplRepo (as_trait = dyn MyRepo)
定义组件 Consumer { repo: Arc<dyn MyRepo> }  // 必选 trait 注入
验证正常注入；验证未注册时启动失败（含错误消息）
```

### Test 4.2 — Prototype shutdown 测试

```
Prototype 组件 + #[component(shutdown)] + 全局计数器
创建实例 → 使用 → drop → shutdown → 验证计数器增加
```

### Test 4.3 — Concurrent trait injection

```
多个线程同时调用 inject_trait_from_store 和 inject_all_traits_from_store
验证无 data race、无死锁、结果正确
```

### Test 4.4 — inner_init 回调测试

```
#[component(init)] 组件 + 在 init 回调中操作 store
验证 inner_init 在 build 之后被调用
```

### Test 4.5 — 错误路径测试

```
损坏的 TOML 文件 → 期望 Result::Err
依赖不存在 → 期望明确的错误消息包含组件名和已注册列表
循环依赖 → 期望错误消息列出环路
```

---

## 执行路线图

```
Week 1: 第 0 批 (P0) ──────── 消除 unsafe + 泛型处理
        第 1 批 (P1) 开始 ─── Prototype lifecycle + 拓扑逆序
Week 2: 第 1 批 (P1) 完成 ─── optional trait 修复 + #[intercept] 修复
        + #[tx_cst] 优先修复 + config panic 消除
Week 3: 第 2 批 (P2) 开始 ─── 类型检测 + DashMap 改造 + 死锁修复
        + 宏卫生 + 非 Arc 编译期报错 + 配置严格模式
Week 4: 第 2 批 (P2) 完成 ─── OptionalInject + AOP 增强 + 条件装配
        第 3 批 (P3) 开始 ─── 性能 + 质量 + shutdown 幂等
Week 5: 第 3 批 (P3) 完成 ─── async_init 分层并行 + comp_run 优化 + 死代码清理
        第 4 批 ────────────── 测试补全 + 回归跑绿
```

**总预估**: 15-25 个工作日（新增 P1 静默错误修复、P2 宏卫生/配置系统等，取决于是否需要同步更新所有示例和插件）。
