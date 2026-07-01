# tx_di v2 架构重构

## 完成内容

将 tx_di DI 框架从"函数指针表"架构重构为"Trait 驱动"架构。

### 核心变更

1. **Component trait** 替代 ComponentDescriptor
   - `type Deps: DepsTuple` — 依赖在类型层面声明
   - `fn build(deps: Self::Deps) -> Self` — 纯函数构造
   - 生命周期钩子（inner_init / init / async_init / async_run / shutdown）全部有默认实现

2. **ComponentMeta 瘦身**（10 字段 → 12 字段，但结构更清晰）
   - 核心字段：type_id, name, dep_type_ids, factory, scope, impl_traits, trait_impls
   - 生命周期函数指针：init_sort_fn, init_fn, async_init_fn, async_run_fn, shutdown_fn
   - 由宏生成，内部调用 Component trait 方法，解决类型擦除问题

3. **`#[derive(Component)]` 宏** 替代 `#[tx_comp]` 属性宏
   - derive 宏不重新输出结构体（只追加 impl + linkme 注册）
   - 辅助属性：`#[component(...)]` 和 `#[tx_cst(...)]` 作为 derive helper attribute
   - 支持：scope, init, conf, as_trait 参数

4. **目录结构重组**
   ```
   tx-di-core/src/
   ├── lib.rs          # crate 入口 + re-export
   ├── component.rs    # Component trait + DepsTuple
   ├── store.rs        # Store + CompRef + trait 注入
   ├── registry.rs     # ComponentMeta + linkme 收集
   ├── topology.rs     # 拓扑排序（Kahn 算法）
   ├── lifecycle.rs    # BuildContext + App
   ├── config.rs       # AppAllConfig
   ├── scope.rs        # Scope enum
   ├── error.rs        # InjectError + RegistryError
   └── aop.rs          # Interceptor trait + 常用拦截器
   ```

5. **AOP 拦截器** 基础设施
   - `Interceptor` trait（before / after 钩子）
   - `InterceptorChain` 链式调用
   - 内置 `LoggingInterceptor` 和 `MetricsInterceptor`

### 测试结果

5 个集成测试全部通过：
- test_basic_inject — 无依赖注入
- test_dependency_inject — 依赖链注入
- test_custom_value — `#[tx_cst]` 自定义值
- test_skip_field — `#[tx_cst(skip)]` 跳过字段
- test_singleton_scope — Singleton 作用域验证

### 关键设计决策

1. **DepsTuple 用元组实现** — 编译期类型安全，最多 16 个依赖
2. **生命周期函数指针** — ComponentMeta 类型擦除后仍能调用 Component trait 方法
3. **derive 宏而非属性宏** — 不重新输出结构体，避免重复定义
4. **panic 而非 Result** — 注入失败是编程错误，启动时 panic 是正确行为
