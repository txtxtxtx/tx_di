# tx_di

基于 `proc_macro` + `linkme` 的 Rust 依赖注入框架。**编译期收集元数据，运行期自动拓扑排序并注入**，零反射、零运行时扫描开销。

[![Crates.io](https://img.shields.io/crates/v/tx-di-core.svg)](https://crates.io/crates/tx-di-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

- **GitHub**: https://github.com/txtxtxtx/tx_di.git  
- **Gitee**: https://gitee.com/tian_xiong/tx_di.git

---

## 目录

- [特性一览](#特性一览)
- [快速上手](#快速上手)
- [核心概念](#核心概念)
  - [作用域 Scope](#作用域-scope)
  - [字段声明方式](#字段声明方式)
  - [配置组件 conf](#配置组件-conf)
  - [自定义初始化 CompInit](#自定义初始化-compinit)
- [BuildContext API](#buildcontext-api)
- [App（固化上下文）](#app固化上下文)
- [配置文件加载](#配置文件加载)
- [内置插件](#内置插件)
  - [tx_di_log — 日志插件](#tx_di_log--日志插件)
  - [tx_di_axum — Web 插件](#tx_di_axum--web-插件)
  - [tx_di_sip — SIP 协议栈插件](#tx_di_sip--sip-协议栈插件)
  - [tx_di_gb28181 — GB28181 服务端插件](#tx_di_gb28181--gb28181-服务端插件)
  - [tx_di_gb28181_client — GB28181 设备端插件](#tx_di_gb28181_client--gb28181-设备端插件)
  - [tx_di_can — CAN/CANFD 插件](#tx_di_can--cancanfd-插件)
- [公共库 tx_gb28181](#公共库-tx_gb28181)
- [架构原理](#架构原理)
- [约束与注意事项](#约束与注意事项)
- [测试](#测试)

---

## 特性一览

| 特性 | 说明 |
|------|------|
| **零反射** | 依赖关系在编译期由宏生成，链接器通过 `linkme` 收集 |
| **Singleton / Prototype** | 两种作用域，scope 标记在**被注入者**上，消费者无感知 |
| **统一 store-based 注入** | `build(store: &DashMap)` 统一签名，App 阶段也支持 Prototype |
| **自动拓扑排序** | Kahn 算法，运行时自动解析构建顺序，循环依赖立即报错 |
| **自定义值注入** | `#[tx_cst(expr)]` 支持任意 Rust 表达式，不进入依赖图 |
| **TOML 配置加载** | `#[tx_comp(conf)]` 自动从配置文件反序列化组件 |
| **生命周期回调** | `CompInit` trait 支持 `inner_init` / 同步 `init` / 异步 `async_init` |
| **并发安全** | 使用 `DashMap` 存储实例，`Arc<T>` 共享，线程安全 |
| **插件化** | 日志、Web、SIP、GB28181、CAN 等开箱即用 |

---

## 快速上手

### 1. 添加依赖

```toml
[dependencies]
tx-di-core = "0.1"
linkme = "0.3"
tokio = { version = "1", features = ["full"] }
```

### 2. 定义并注入组件

```rust
use std::sync::Arc;
use tx_di_core::{tx_comp, tx_cst, BuildContext};

// ── 无依赖单例 ────────────────────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]                          // 默认 Singleton
pub struct DbPool;

// ── 带自定义值的单例 ──────────────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct AppConfig {
    #[tx_cst("my-app".to_string())]
    pub app_name: String,

    #[tx_cst(8080u16)]
    pub port: u16,
}

// ── 原型组件（每次注入创建新实例）────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp(scope = Prototype)]
pub struct RequestLogger {
    #[tx_cst("[REQ]".to_string())]
    pub prefix: String,
}

// ── 依赖其他组件的服务 ────────────────────────────────────────────
#[derive(Clone, Debug)]
#[tx_comp]
pub struct UserService {
    pub db: Arc<DbPool>,            // 自动注入 DbPool 单例
    pub config: Arc<AppConfig>,     // 自动注入 AppConfig 单例
}

// ── main ──────────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    // 创建上下文，自动扫描所有 #[tx_comp] 组件并按拓扑顺序构建
    let ctx = BuildContext::new::<std::path::PathBuf>(None);

    let svc = ctx.inject::<UserService>();
    println!("app_name: {}", svc.config.app_name);
    println!("port:     {}", svc.config.port);
}
```

---

## 核心概念

### 作用域 Scope

| 作用域 | 宏写法 | 行为 |
|--------|--------|------|
| **Singleton**（默认） | `#[tx_comp]` | 全局唯一，首次注入时构建并缓存 `Arc<T>` |
| **Prototype** | `#[tx_comp(scope = Prototype)]` 或 `#[tx_comp(scope)]` | 每次 `inject()` 调用工厂创建全新实例 |

> **关键原则**：scope 标记在**被注入者**上，消费者只需写 `Arc<T>`，框架自动处理。

```rust
// ✅ Prototype 标记在 RequestLogger 自己身上
#[tx_comp(scope = Prototype)]
pub struct RequestLogger { ... }

// 消费者无需关心 scope
#[tx_comp]
pub struct AppServer {
    pub logger: Arc<RequestLogger>,  // 每次创建新的 logger 实例
}
```

> **v0.1.7 新特性**：build() 后的 `App` 阶段也支持 Prototype 注入。

```rust
let app = ctx.build()?;
// App 阶段同样可以注入 Prototype
let l1 = app.inject::<RequestLogger>();
let l2 = app.inject::<RequestLogger>();
assert_ne!(Arc::as_ptr(&l1), Arc::as_ptr(&l2));  // 不同实例
```

---

### 字段声明方式

| 写法 | 语义 |
|------|------|
| `field: Arc<T>` | 从 DI 容器注入，框架根据 `T` 的 scope 自动处理 |
| `field: T`（其他类型）| 非 `Arc` 包裹时，宏仍会尝试 `ctx.inject::<T>()` |
| `#[tx_cst(expr)]` | **不走 DI**，直接用表达式赋值，不计入依赖图 |
| `Option<T>` 字段 | 自动设为 `None`，不参与依赖注入 |
| `#[tx_cst(skip)]` | 跳过注入，使用 `Default::default()` |

```rust
#[tx_comp]
pub struct MyService {
    // ① DI 注入
    pub db: Arc<DbPool>,

    // ② 自定义值：任意表达式，不进依赖图
    #[tx_cst("0.0.0.0:8080".to_string())]
    pub bind_addr: String,

    // ③ 调用函数
    #[tx_cst(default_headers())]
    pub headers: HashMap<String, String>,

    // ④ 集合
    #[tx_cst(Vec::new())]
    pub items: Vec<String>,

    // ⑤ 读取环境变量
    #[tx_cst(std::env::var("SECRET").unwrap_or_default())]
    pub secret: String,
}
```

> `#[tx_cst(expr)]` 字段**不会**被加入 `DEP_IDS`，不影响拓扑排序。

---

### 配置组件 conf

用 `#[tx_comp(conf)]` 标记的组件会自动从 TOML 配置文件中加载配置，无需手动解析。

```toml
# configs/app.toml
[app_config]
app_name = "production-app"
port = 9090
```

```rust
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
#[tx_comp(conf)]        // 自动从 [app_config] 段加载
pub struct AppConfig {
    #[serde(default = "default_name")]
    pub app_name: String,

    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_name() -> String { "my-app".to_string() }
fn default_port() -> u16 { 8080 }
```

**自定义配置键**：

```rust
#[tx_comp(conf = "server")]   // 从 TOML 的 [server] 段读取
pub struct MyComponent { ... }
```

**加载配置文件**：

```rust
let ctx = BuildContext::new(Some("configs/app.toml"));
let ctx = BuildContext::new::<std::path::PathBuf>(None);  // 不使用配置文件
```

---

### 自定义初始化 CompInit

在所有依赖构建完成后，可以通过 `CompInit` trait 执行同步或异步初始化逻辑。

```rust
use tx_di_core::{tx_comp, CompInit, App, BoxFuture, RIE};
use std::sync::Arc;

#[derive(Debug)]
#[tx_comp(init)]        // init flag：告知宏"我自己写 CompInit 实现"
pub struct AppServer {
    pub user_svc: Arc<UserService>,
    pub bind_addr: String,
}

impl CompInit for AppServer {
    fn init(ctx: Arc<App>) -> RIE<()> {
        println!("同步初始化");
        Ok(())
    }

    fn async_init(ctx: Arc<App>) -> BoxFuture<'static, RIE<()>> {
        let addr = ctx.inject::<AppServer>().bind_addr.clone();
        Box::pin(async move {
            println!("异步初始化，bind_addr: {}", addr);
            Ok(())
        })
    }

    fn init_sort() -> i32 { 1000 }
}
```

> **组件内部初始化（`inner_init`）**：在组件构建时（`build()` 内部）立即调用，早于 `init()`。  
> 注意：`inner_init` 仅在 `BuildContext` 构建阶段调用，App 阶段跳过。

---

## BuildContext API

`BuildContext` 是构建阶段的主要入口：

```rust
use std::path::PathBuf;
use tx_di_core::BuildContext;

// 创建上下文（自动扫描 + 拓扑排序 + 注册所有 #[tx_comp] 组件）
let ctx = BuildContext::new(Some("configs/app.toml"));
let ctx = BuildContext::new::<PathBuf>(None);

// ── 注入 ──────────────────────────────────────────────────────────

// inject: 根据 scope 自动处理（Singleton 缓存 Arc，Prototype 创建新实例）
let db: Arc<DbPool> = ctx.inject::<DbPool>();
let logger: Arc<RequestLogger> = ctx.inject::<RequestLogger>();

// get: 自动区分 Singleton / Prototype
let db: Arc<DbPool> = ctx.get::<DbPool>();

// get_singleton: 无需 ComponentDescriptor 约束
let db: Arc<DbPool> = ctx.get_singleton::<DbPool>();

// try_get_singleton: 安全版本，未找到返回 None
let db: Option<Arc<DbPool>> = ctx.try_get_singleton::<DbPool>();

// take: 移交所有权（仅 Singleton）
let server: AppServer = ctx.take::<AppServer>().expect("未找到 AppServer");

// ── 调试 ──────────────────────────────────────────────────────────

BuildContext::debug_registry();     // 打印所有注册组件及依赖关系
println!("组件数: {}", ctx.len());

// ── 构建固化上下文 ──────────────────────────────────────────────

let app: App = ctx.build()?;
ctx.build_and_run().await?;         // build + init 的快捷方式
```

---

## App（固化上下文）

`App` 是 `BuildContext::build()` 之后的"固化"状态，通过 `Arc<App>` 可在多线程中共享。

```rust
use std::sync::Arc;
use tx_di_core::{BuildContext, App};

let ctx = BuildContext::new::<std::path::PathBuf>(None);
let app = Arc::new(ctx.build()?);

// Singleton：返回缓存实例
let db = app.inject::<DbPool>();
let db = app.try_inject::<DbPool>();  // Option 安全版本

// Prototype（v0.1.7+）：每次创建新实例
let l1 = app.inject::<RequestLogger>();
let l2 = app.try_inject::<RequestLogger>();

println!("组件数: {}", app.len());
```

> **v0.1.7 升级**：App 阶段现在完整支持 Prototype 注入，`try_inject` 也能返回 Prototype。

---

## 配置文件加载

```toml
# configs/app.toml
[app_config]
app_name = "my-app"
port = 8080

[log_config]
level = "info"
dir = "./logs"
console_output = true

[web_config]
host = "0.0.0.0"
port = 8080
enable_cors = true
max_body_size = 10485760    # 10MB

[sip_config]
host = "::"
port = 5060
```

### 访问全局配置 AppAllConfig

```rust
let global_cfg = ctx.inject::<tx_di_core::AppAllConfig>();

// 读取指定键
let app_name: Option<String> = global_cfg.get("app_config.app_name");
let port: Option<u16>        = global_cfg.get("app_config.port");

// 带默认值读取
let retention = global_cfg.get_or_default("log_config.retention_days", 90u64);
```

---

## 内置插件

插件通过 `linkme` 在编译期自动注册组件，使用时**必须在代码中 `use` 导入**，否则链接器会优化掉。

### tx_di_log — 日志插件

基于 `tracing` + `tracing-subscriber`，支持按天滚动文件日志、控制台彩色输出、模块级别过滤。

```toml
[dependencies]
tx_di_log = "0.1"
```

```rust
use tx_di_core::BuildContext;
use tx_di_log;  // 必须导入

#[tokio::main]
async fn main() {
    let ctx = BuildContext::new(Some("configs/app.toml"));
    tracing::info!("应用启动");
}
```

配置示例：

```toml
[log_config]
level = "info"
dir = "./logs"
console_output = true
time_format = "local"
retention_days = 90
prefix = "my-app"

[log_config.modules]
"my_app::db" = "debug"
"noisy_lib"  = "warn"
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `level` | String | `"info"` | 全局日志级别 |
| `dir` | String | `"./logs"` | 日志文件目录 |
| `console_output` | bool | `false` | 是否输出到控制台 |
| `time_format` | String | `"utc"` | 时间格式 |
| `retention_days` | u64 | `90` | 日志保留天数 |
| `prefix` | String | `"tx_di"` | 日志文件名前缀 |

---

### tx_di_axum — Web 插件

基于 `axum` + `tokio` 的异步 HTTP 服务器，支持 CORS、静态文件、SPA、中间件链路。

```toml
[dependencies]
tx_di_axum = "0.1"
```

```rust
use tx_di_core::BuildContext;
use tx_di_axum;
use tx_di_log;

#[tokio::main]
async fn main() {
    let ctx = BuildContext::new(Some("configs/app.toml"));
    ctx.build_and_run().await.expect("启动失败");
    tokio::signal::ctrl_c().await.ok();
}
```

```toml
[web_config]
host = "0.0.0.0"
port = 8080
enable_cors = true
max_body_size = 10485760
timeout_secs = 30
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | String | `"127.0.0.1"` | 监听地址（IPv4 / IPv6） |
| `port` | u16 | `8080` | 监听端口 |
| `enable_cors` | bool | `false` | 是否启用 CORS |
| `max_body_size` | usize | `10485760` | 最大请求体 |
| `timeout_secs` | u64 | `30` | 请求超时 |

内置端点：`GET /health` 健康检查。

---

### tx_di_sip — SIP 协议栈插件

基于 `rsipstack 0.5` 的 SIP 协议栈包装，提供 SIP 消息路由能力。

```toml
[dependencies]
tx_di_sip = "0.1"
```

**核心功能**：
- `SipPlugin` — SIP 传输层（UDP/TCP 监听）
- `SipRouter` — 消息处理器注册与分发
- 支持 REGISTER / INVITE / MESSAGE / NOTIFY / OPTIONS 等标准 SIP 方法

**配置**：

```toml
[sip_config]
host = "::"        # 监听地址
port = 5060        # 监听端口
transport = "udp"  # udp / tcp / both
```

---

### tx_di_gb28181 — GB28181 服务端插件

基于 `tx_di_sip` 构建的 GB28181-2022 上级平台完整服务端。

**核心功能（74 个功能点）**：

| 功能 | 说明 |
|------|------|
| **设备注册管理** | REGISTER/注销/心跳，支持 SIP 摘要认证 |
| **目录查询** | Catalog 查询，完整解析通道列表（含经纬度） |
| **实时点播** | INVITE s=Play，联动流媒体分配 RTP 端口 |
| **历史回放** | INVITE s=Playback，含时间范围 SDP |
| **PTZ 云台控制** | 8 方向 + 变倍 + 聚焦 + 光圈 |
| **录像控制/查询** | 开始/停止录像，录像文件列表查询 |
| **报警事件** | NOTIFY 报警接收 |
| **广播/对讲** | 语音广播邀请/接收，对讲音频会话 |
| **移动位置** | GPS 定位上报通知 |
| **远程启动** | TeleBoot 远程重启 |
| **拉框缩放** | ZoomIn / ZoomOut |
| **巡航轨迹** | 巡航轨迹列表/详情查询 |
| **PTZ 精准控制** | 绝对位置云台控制（2022 新增） |
| **存储管理** | 存储卡格式/状态查询 |
| **目标跟踪** | 目标跟踪控制（2022 新增） |
| **流媒体后端** | ZLM / MediaMTX 统一后端接入 |

---

### tx_di_gb28181_client — GB28181 设备端插件

模拟 GB28181 设备端的插件，实现向平台注册、响应查询、处理点播。

```rust
use tx_di_core::BuildContext;
use tx_di_gb28181_client;

#[tokio::main]
async fn main() {
    let ctx = BuildContext::new(Some("configs/gb28181-client.toml"));
    ctx.build_and_run().await?;
}
```

**功能**：
- REGISTER 向平台注册（含摘要认证）
- 心跳保活
- 响应目录查询、设备信息查询、状态查询
- 实时点播 INVITE SDP answer
- 可配置的通道列表和预置位

---

### tx_di_can — CAN/CANFD 插件

嵌入式 CAN 总线通信插件，支持 SocketCAN / PCAN 双适配器，集成 ISO-TP 和 UDS 刷写。

```toml
[dependencies]
tx_di_can = "0.1"
```

**核心能力**：
- `SocketCAN` 适配器（Linux PF_CAN）
- `PCAN` 适配器（Windows pcanbasic.dll）
- ISO-TP（ISO 15765-2）多帧传输
- UDS（ISO 14229）诊断协议
- 刷写引擎（带安全和完整性校验）

---

## 公共库 tx_gb28181

`tx_gb28181` 是 GB28181-2022 协议公共库，供服务端和客户端插件共享。

| 模块 | 内容 |
|------|------|
| `device` | 设备/通道数据类型（`DeviceInfo` / `ChannelInfo` / `ChannelStatus`） |
| `cmd_type` | 协议指令枚举 `Gb28181CmdType`（15+ 种 CmdType） |
| `event` | 事件类型 `Gb28181Event`（27 种） + 全局广播 `subscribe` / `emit` |
| `xml` | MANSCDP XML 构建与解析 |
| `sdp` | SDP 构建与解析（含 IPv4/IPv6 双栈） |
| `sip` | SIP URI 解析工具 |

```rust
use tx_gb28181::{Gb28181Event, Gb28181CmdType, DeviceInfo, ChannelInfo};

// 订阅全局事件
let mut rx = tx_gb28181::event::subscribe();
tokio::spawn(async move {
    while let Ok(ev) = rx.recv().await {
        match ev {
            Gb28181Event::DeviceRegistered { device_id, .. } => println!("上线: {}", device_id),
            _ => {}
        }
    }
});
```

---

## 架构原理

```
用户代码（定义组件）
  #[tx_comp]                 struct DbPool {}
  #[tx_comp(scope=Prototype)] struct Logger { #[tx_cst(...)] prefix: String }
  #[tx_comp]                 struct AppServer { db: Arc<DbPool>, log: Arc<Logger> }
         │
         │  proc_macro 展开（编译期）
         ▼
tx-di-macros
  1. 解析 scope 参数   →  Singleton / Prototype
  2. 解析字段注入方式   →  Inject / Custom / Optional / Skip
  3. 生成 ComponentDescriptor impl（DEP_IDS + SCOPE + build()）
  4. 生成 CompInit 默认空实现
  5. 注册 linkme distributed_slice 条目（ComponentMeta）
         │
         │  链接器合并 link section（运行前）
         ▼
tx-di-core（运行时）
  COMPONENT_REGISTRY    全局静态组件元数据切片（linkme 收集）
  BuildContext::new()
    ├─ 创建 AppAllConfig 单例（加载 TOML）
    └─ auto_register_all()
         ├─ 从 COMPONENT_REGISTRY 收集所有 ComponentMeta
         ├─ topo_sort()（Kahn 算法）检测循环依赖
         └─ 按顺序调用 factory(store)，Singleton 立即缓存
  BuildContext::build()
    ├─ 按 init_sort 顺序调用 init()（同步）
    ├─ 并行调用所有 async_init()（异步）
    └─ 转移 store → App（App 也支持 Prototype 注入）
```

### v0.1.7 内部变更

| 变更 | 说明 |
|------|------|
| `build` 签名 | `fn build(ctx: &mut BuildContext)` → `fn build(store: &DashMap<TypeId, CompRef>)` |
| `inject` 签名 | `inject(&mut self)` → `inject(&self)`，不再需要可变引用 |
| `CompRef::Factory` | 接收 `&DashMap` 而非 `&mut BuildContext` |
| App 阶段 | 现在完整支持 Singleton 和 Prototype 注入 |
| `inject_from_store` | 新增公共辅助函数，供宏生成的 build 方法调用 |

---

## 约束与注意事项

| 约束 | 原因 |
|------|------|
| 组件需 `T: Send + Sync + 'static` | 存入 `Arc<dyn Any + Send + Sync>`，支持多线程 |
| 配置组件需 `Deserialize + Default` | 配置键不存在时使用 serde 默认值 |
| `take()` 仅用于 Singleton | Prototype 无缓存，无法取出所有权 |
| 插件 crate 必须 `use` 导入 | `linkme` 依赖链接器，未引用的 crate 会被优化掉 |
| 避免循环依赖 | 拓扑排序时检测，存在则 panic 并列出循环节点 |
| `async_init` 返回 `'static` Future | 不能在 async 块中直接借用 `ctx` |
| `inner_init` 仅在 BuildContext 阶段调用 | App 阶段跳过（无 BuildContext 可用） |

---

## 测试

```bash
# 运行所有测试
cargo test

# 只运行 DI 核心测试（28+ 个，含 6 个 Prototype 测试）
cargo test -p di-example

# 只运行 GB28181 相关测试（70+ 个）
cargo test -p tx_gb28181 -p tx_di_gb28181
```

---

## 许可证

MIT — 详见 [LICENSE](LICENSE)
