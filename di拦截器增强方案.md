# 拦截器（AOP）增强方案对比

> 当前 AOP 系统的缺陷分析与可行方案。

---

## 现状

```rust
// ── 组件声明 ──
#[derive(Component)]
#[component(intercept(AuthInterceptor, LogInterceptor))]
pub struct UserService;

impl UserService {
    #[intercept]
    pub fn get_user(&self, id: u64) -> RIE<User> { ... }
}

// ── 拦截器链存储 ──
// key = Arc::as_ptr(comp) as usize  → 全局 Mutex<HashMap<usize, Arc<InterceptorChain>>>
// init 阶段：set_interceptor_chain(key, chain)
// 方法调用：get_interceptor_chain(key) → before_all → body → after_all
```

## 当前缺陷

| 缺陷 | 严重度 | 说明 |
|------|--------|------|
| **全局 Mutex 瓶颈** | 高 | 每次方法调用过一把全局锁，高 QPS 下竞争热点 |
| **内存泄漏** | 高 | key 是裸地址从不删除，App/组件销毁后条目永久残留 |
| **ABA 风险** | 中 | 旧实例释放→新实例同地址→命中旧拦截链 |
| **Mutex Poison 传播** | 中 | 任一线程 panic → 全局锁 poisoned → 所有拦截器不可用 |
| **无 around 拦截器** | 中 | 仅 before/after，无法替换返回或包装调用 |
| **等值参数表示有限** | 低 | `ArgValue` 仅 i64/Str/Bool/Other，`Other` 通过 Debug 字符串传递 |

---

## 方案 A：组件内嵌 OnceLock（推荐）

### 思路

拦截器链直接内嵌到组件结构体中，不做全局存储：

```rust
// 宏为组件结构体自动生成隐藏字段
#[derive(Component)]
#[component(intercept(AuthInterceptor))]
pub struct UserService {
    pub db: Arc<DbPool>,
    // ↓ 宏自动生成
    __interceptor_chain: OnceLock<Arc<InterceptorChain>>,
}

// 方法调用不再查全局表
impl UserService {
    #[intercept]
    pub fn get_user(&self, id: u64) -> RIE<User> {
        let chain = self.__interceptor_chain.get()
            .expect("拦截器链未初始化");
        // ... before → body → after
    }
}

// init 阶段填充 OnceLock（不再写全局表）
fn init(app: &Arc<App>) -> RIE<()> {
    let comp = inject_from_store::<Self>(&app.store);
    let mut chain = InterceptorChain::new();
    chain.push_arc(inject_from_store::<AuthInterceptor>(&app.store));
    comp.__interceptor_chain.set(Arc::new(chain))
        .map_err(|_| AppError::...)?;
    Ok(())
}
```

### 改动点

| 层 | 改动 |
|----|------|
| `component_impl.rs` | 为有 `intercept(...)` 的组件生成隐藏字段 `__interceptor_chain: OnceLock<Arc<InterceptorChain>>` |
| `intercept_macro.rs` | `self.__interceptor_chain.get().unwrap()` 替代 `get_interceptor_chain(key)` |
| `intercept.rs`（codegen） | `init` 覆写中 `comp.__interceptor_chain.set(chain)` 替代 `set_interceptor_chain(key, chain)` |
| `aop.rs` | **删除** `INTERCEPTOR_CHAINS` / `chains_map` / `set_interceptor_chain` / `get_interceptor_chain` |

### 优劣

| 优点 | 缺点 |
|------|------|
| 完全消除全局表：无锁、无泄漏、无 ABA | 组件增加一个 `OnceLock` 字段的内存开销（8 字节） |
| 拦截链生命周期与组件一致 — drop 即清理 | `OnceLock` 需 `#[derive(Component)]` 宏生成隐藏字段（已有 precedence） |
| 方法调用只需一次 `.get().unwrap()` — O(1) 无锁 | 用户可见结构体增加隐藏字段（但 `#[derive]` 已默认不可手动构造） |

---

## 方案 B：DashMap 全局表 + 生命周期清理

### 思路

保持全局表架构，但：`Mutex<HashMap>` → `DashMap`；App.shutdown 时清理 entries。

```rust
// aop.rs
static INTERCEPTOR_CHAINS: OnceLock<DashMap<usize, Arc<InterceptorChain>>> = OnceLock::new();

pub fn set_interceptor_chain(key: usize, chain: Arc<InterceptorChain>) {
    chains_map().insert(key, chain);
}

pub fn get_interceptor_chain(key: usize) -> Option<Arc<InterceptorChain>> {
    chains_map().get(&key).map(|r| r.clone())
}

pub fn remove_interceptor_chain(key: usize) {
    chains_map().remove(&key);
}
```

shutdown 时逐组件清理（`ComponentMeta` 记录所有需要清理的 key）。

### 改动点

| 层 | 改动 |
|----|------|
| `aop.rs` | `Mutex<HashMap>` → `DashMap`，新增 `remove_interceptor_chain` |
| `lifecycle.rs` | shutdown 中清理拦截器链 entry |
| `registry.rs` | `ComponentMeta` 记录拦截器 key（如有） |

### 优劣

| 优点 | 缺点 |
|------|------|
| 改动最少（~30 行） | ABA 风险仍在（同一地址复用），概率低但非零 |
| 无锁读，性能好 | 清理逻辑需 ComponentMeta 记录 key |
| 解决 poison 问题 | 全局表依然存在 |

---

## 方案 C：tower-like Service 层叠

### 思路

将组件建模为 `Service<Request>`，拦截器建模为 `Layer`，编译期构建洋葱栈：

```rust
// 组件 = Service
trait Service<Req> {
    type Response;
    type Error;
    async fn call(&self, req: Req) -> Result<Self::Response, Self::Error>;
}

// 拦截器 = Layer
trait Interceptor<Req, S: Service<Req>>: Service<Req> {
    fn wrap(inner: S) -> Self;
}

// 方法 = 自动生成 Service 实现
#[derive(Component)]
#[component(intercept(LogLayer, AuthLayer))]
pub struct UserService { ... }

// 生成：
// type GetUserStack = LogLayer<AuthLayer<UserServiceGetUserImpl>>;
```

### 优劣

| 优点 | 缺点 |
|------|------|
| 编译期类型安全、零运行时开销 | 重写量极大，与现有 API 完全不兼容 |
| `around` / 完全替换返回天然支持 | 不支持异构方法（每方法独立 impl） |
| 生态兼容性（tower middleware 可复用） | 学习曲线陡 |

---

## 方案对比

| 维度 | A：内嵌 OnceLock | B：DashMap + 清理 | C：tower-like |
|------|:---:|:---:|:---:|
| 全局表 | ❌ 消除 | ✅ 保留 | ❌ 消除 |
| 内存泄漏 | ✅ 消除 | ✅ 清理 | ✅ 消除 |
| ABA 风险 | ✅ 消除 | ⚠️ 残留 | ✅ 消除 |
| Mutex/Poison | ✅ 消除 | ✅ DashMap | ✅ 消除 |
| 改动量 | 中（~150 行） | 小（~50 行） | 大（~800 行+） |
| API 兼容 | 完全兼容 | 完全兼容 | 不兼容 |
| around 扩展 | 易 | 易 | 天然支持 |
| 实现复杂度 | 低 | 低 | 高 |

---

## 建议

**选择方案 A（组件内嵌 OnceLock）**。

---

## 方案 A 细化

### 一、隐藏字段生成

`component_impl.rs` 中，对声明了 `intercept(...)` 的组件，在 `build()` 输出的结构体末尾追加隐藏字段：

```rust
// 输入
#[derive(Component)]
#[component(intercept(AuthInterceptor, LogInterceptor))]
pub struct UserService {
    pub db: Arc<DbPool>,
}

// 输出（build 函数生成）
Self {
    db: deps.0.clone(),
    #[allow(non_snake_case)]
    __interceptor_chain: ::std::sync::OnceLock::new(),  // ← 宏追加
}
```

**关键设计**：
- 字段名采用 `__` 前缀避免与用户字段冲突，同时加 `#[allow(non_snake_case)]`
- 隐藏字段类型 `OnceLock<Arc<InterceptorChain>>` 直接硬编码，不暴露为 `FieldKind` 变体
- `build()` 中对该字段输出 `OnceLock::new()`，空壳占位
- `inner_init` 不处理此字段（由后续的 `init` 或 `app_init` 填充）

### 二、init 阶段注入拦截器链

`intercept.rs`（codegen）为有 `intercept(...)` 的组件覆写 `init`：

```rust
// 拦截器类型列表来自 #[component(intercept(AuthInterceptor, LogInterceptor))]
fn app_init(comp: Arc<Self>, app: &Arc<App>) -> RIE<()> {
    let mut chain = ::tx_di_core::aop::InterceptorChain::new();
    // 按声明顺序 push 拦截器实例
    chain.push_arc(::tx_di_core::inject_from_store::<AuthInterceptor>(&app.store));
    chain.push_arc(::tx_di_core::inject_from_store::<LogInterceptor>(&app.store));
    // 填充 OnceLock（仅一次，重复 set 会返回 Err — 此处不应发生）
    comp.__interceptor_chain
        .set(::std::sync::Arc::new(chain))
        .map_err(|_| ::tx_di_core::AppError::with_context(
            ::tx_di_core::DiErr::InjectError,
            "拦截器链已初始化（疑似 init 被多次调用）"
        ))?;
    Ok(())
}
```

**关键设计**：
- 拦截器实例通过 `inject_from_store::<I>` 从 DI 容器中获取（拦截器本身也是 Component，可享受 DI 好处）
- `push_arc` 直接复用已有 `Arc<I>`，避免额外分配
- `OnceLock::set()` 保证仅写一次，重复调用返回 `Err` 并传播

### 三、方法调用端

`intercept_macro.rs` 中，`#[intercept]` 方法改用 `self.__interceptor_chain`：

```rust
// 旧：let __key = self as *const Self as usize;
//     let __chain = get_interceptor_chain(__key).expect(...);

// 新：
let __chain = self.__interceptor_chain.get()
    .expect("拦截器链未初始化：请确认 init 阶段已执行");
```

`aop.rs` 中 **删除**全局静态 `INTERCEPTOR_CHAINS`、`chains_map()`、`set_interceptor_chain`、`get_interceptor_chain`。

### 四、around 拦截器

#### 4.1 `Interceptor` trait 扩展

在现有 `before_all` / `after_all` 基础上，新增 `around` 接口：

```rust
/// 拦截器 trait
pub trait Interceptor: Send + Sync + 'static {
    /// before 拦截（前置处理）
    fn before(&self, ctx: &CallContext) -> Result<(), AppError> { Ok(()) }

    /// after 拦截（后置处理，可修改返回值）
    fn after(&self, ctx: &CallContext, result: &mut CallResult) { }

    /// around 拦截（完全包裹调用）
    ///
    /// 默认委托给 call 执行业务逻辑，等价于"不拦截"。
    /// 覆写此方法可完全替换业务逻辑返回、短路、包装等。
    fn around(&self, ctx: &CallContext, call: BoxCall) -> CallResult {
        call.execute()
    }
}

/// 可执行调用（类似 FnOnce）
pub trait CallFn: Send {
    fn execute(self: Box<Self>) -> CallResult;
}

pub type BoxCall = Box<dyn CallFn>;
```

#### 4.2 使用方式

命令式（同步/异步均可使用同一 trait）：

```rust
// 示例 1：短路 — 鉴权失败直接返回
impl Interceptor for AuthInterceptor {
    fn around(&self, ctx: &CallContext, call: BoxCall) -> CallResult {
        if !self.has_permission(ctx) {
            return CallResult::Err(AppError::with_context(DiErr::... , "auth failed"));
        }
        call.execute()
    }
}

// 示例 2：计时包装
impl Interceptor for MetricsInterceptor {
    fn around(&self, ctx: &CallContext, call: BoxCall) -> CallResult {
        let start = std::time::Instant::now();
        let result = call.execute();
        self.record(ctx.method, start.elapsed());
        result
    }
}

// 示例 3：重试
impl Interceptor for RetryInterceptor {
    fn around(&self, ctx: &CallContext, call: BoxCall) -> CallResult {
        for attempt in 0..3 {
            if let Ok(r) = call.execute().into_result() { return Ok(r); }
        }
        CallResult::Err(...)
    }
}
```

#### 4.3 洋葱链执行

`InterceptorChain` 的 `around_all` 方法构建洋葱调用链：

```rust
impl InterceptorChain {
    pub fn around_all(&self, ctx: &CallContext, exec: BoxCall) -> CallResult {
        // 叠装洋葱：最外层拦截器最先执行
        let mut call = exec;
        for interceptor in self.interceptors.iter().rev() {
            let inner = call;
            let ic = interceptor.clone();
            call = Box::new(move || ic.around(ctx, inner));
        }
        call.execute()
    }
}
```

**执行顺序**（以 `#[component(intercept(A, B))]` 为例）：

```
进入 → A.before → B.before → B.around {
                                    A.around {
                                        body
                                    }
                                }
                                → B.after → A.after → 返回
```

### 五、`ArgValue` 类型增强

#### 5.1 当前问题

```rust
pub enum ArgValue {
    I64(i64),
    Str(String),
    Bool(bool),
    Other(String),  // 仅通过 Debug 输出，不可逆还原
}
```

`Other(String)` 通过 `format!("{:?}", arg)` 生成字符串，丢失了原始类型信息，拦截器无法做有意义的类型判断。

#### 5.2 方案：serde 序列化 + TypeId 保留

```rust
use std::any::TypeId;

pub enum ArgValue {
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Bool(bool),
    /// 支持 serde::Serialize 的任意类型
    Serialized {
        /// 原始类型的 TypeId（用于拦截器做类型判断）
        type_id: TypeId,
        /// 类型名称（用于日志/可读性）
        type_name: &'static str,
        /// JSON 序列化后的字符串
        json: String,
    },
}
```

#### 5.3 生成端

`#[intercept]` 方法参数中，对非简单类型用 serde 序列化：

```rust
// 宏生成（以 fn get_user(&self, id: u64, req: UserReq) -> Result<User, AppError> 为例）
let __ctx = CallContext::new(stringify!(get_user))
    .with_arg("id", ArgValue::U64(id))
    .with_arg("req", {
        // 利用 serde_json 序列化（tx-di-core 已有 serde 依赖）
        let json = ::tx_di_core::serde_json::to_string(&req)
            .unwrap_or_else(|_| "<序列化失败>".to_string());
        ArgValue::Serialized {
            type_id: std::any::TypeId::of::<UserReq>(),
            type_name: std::any::type_name::<UserReq>(),
            json,
        }
    });
```

#### 5.4 使用端

```rust
impl Interceptor for AuditInterceptor {
    fn before(&self, ctx: &CallContext) -> Result<(), AppError> {
        let req = ctx.get_arg("req").and_then(|v| {
            if let ArgValue::Serialized { json, type_name, .. } = v {
                if *type_name == std::any::type_name::<UserReq>() {
                    return ::tx_di_core::serde_json::from_str::<UserReq>(json).ok();
                }
            }
            None
        });
        // 审计逻辑...
        Ok(())
    }
}
```

### 六、改动范围汇总

| 层 | 文件 | 改动 |
|----|------|------|
| **隐藏字段** | `codegen/component_impl.rs` | `intercept(...)` 组件在 `build()` 输出末尾追加 `__interceptor_chain: OnceLock::new()` |
| **init 注入** | `codegen/intercept.rs` | 覆写 `init`/`app_init`：resolve 拦截器实例 → push_arc → OnceLock::set |
| **方法调用** | `intercept_macro.rs` | `self.__interceptor_chain.get().unwrap()` 替代 `get_interceptor_chain(key)` |
| **删除全局表** | `aop.rs` | 删除 `INTERCEPTOR_CHAINS` / `chains_map` / `set_` / `get_interceptor_chain` |
| **around** | `aop.rs` | +`CallFn` trait, +`around(&self, ctx, BoxCall) -> CallResult`, +`InterceptorChain::around_all` |
| **ArgValue** | `aop.rs` | +`U64`/`F64`/`Serialized` 变体 |
| **ArgValue 生成** | `intercept_macro.rs` | +serde 序列化非简单类型参数 |
| **around 集成** | `intercept_macro.rs` | 方法生成改为调用 `around_all` 而非 `before_all`+body+`after_all` |
| **test** | `test_component.rs` | 新增：around 短路、around 重试、Serialized 参数反序列化、OnceLock 重复初始化 |

### 七、接口设计一览

```rust
// ── aop.rs ──

pub trait Interceptor: Send + Sync + 'static {
    fn before(&self, ctx: &CallContext) -> Result<(), AppError> { Ok(()) }
    fn after(&self, ctx: &CallContext, result: &mut CallResult) {}
    fn around(&self, ctx: &CallContext, call: BoxCall) -> CallResult { call.execute() }
}

pub trait CallFn: Send { fn execute(self: Box<Self>) -> CallResult; }

impl<F: FnOnce() -> CallResult + Send> CallFn for F {
    fn execute(self: Box<Self>) -> CallResult { self() }
}

pub type BoxCall = Box<dyn CallFn>;

pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    pub fn push_arc(&mut self, ic: Arc<dyn Interceptor>) { ... }
    pub fn around_all(&self, ctx: &CallContext, exec: BoxCall) -> CallResult { ... }
}

pub enum ArgValue {
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Bool(bool),
    Serialized {
        type_id: std::any::TypeId,
        type_name: &'static str,
        json: String,
    },
}
```
