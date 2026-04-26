# tx_di_axum

基于 axum 的 web 服务器插件，为 tx_di 依赖注入框架提供高性能 Web 服务支持。

## ✨ 特性

- 🚀 **高性能**：基于 axum + tokio 异步运行时，支持 IPv4/IPv6 双栈
- ⚙️ **TOML 配置驱动**：通过配置文件灵活定制 Web 服务器参数与中间件
- 🔧 **依赖注入集成**：与 tx_di 框架无缝集成，组件自动注册与初始化
- 🏥 **健康检查端点**：内置 `/health` 路由用于服务监控
- 🌐 **CORS 支持**：通过配置开启跨域资源共享
- 📦 **请求体限制**：可配置的最大请求体大小
- 🧅 **中间件洋葱模型**：支持按优先级排序的动态中间件注册
- 📝 **API 日志中间件**：内置请求/响应日志记录，智能过滤静态资源
- 🗂 **静态文件服务**：支持传统静态文件目录与 SPA 应用托管
- 🔗 **DI 组件提取器**：`DiComp<T>` 在 Handler 中直接注入 DI 组件
- 📊 **统一响应封装**：`R<T>` / `WebErr` 标准化 API 响应与错误处理
- ⏱ **请求超时控制**：可配置的全局请求超时时间
- 🗜 **响应压缩**：可选的 gzip/br 压缩中间件
- 🔍 **链路追踪**：可选的 TraceLayer 链路追踪日志

## 🚀 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
tx_di_axum = { path = "../plugins/tx_di_axum" }
tx-di-core = { path = "../tx-di-core" }
tx_di_log = { path = "../plugins/tx_di_log" }  # 推荐配合日志插件使用
linkme = "0.3"
```

### 2. ⚠️ 导入插件（必须步骤）

```rust
use tx_di_core::{app, BuildContext};
use tx_di_axum;  // ← 必须导入以触发 WebPlugin 组件注册
use tx_di_log;   // ← 推荐配合日志插件

app! { AppModule }
```

**为什么需要这一步？**

tx_di 使用 `linkme` 进行编译期静态注册。如果代码中没有引用 `tx_di_axum`，Rust 链接器会优化掉这个 crate，导致 `WebPlugin` 组件无法注册到 DI 容器。

通过 `use tx_di_axum;` 确保 crate 被链接，触发组件自动注册。

### 3. 创建配置文件

创建 `configs/app.toml`：

#### 基础配置（IPv4）

```toml
[log_config]
level = "info"
console_output = true
time_format = "local"

[web_config]
host = "127.0.0.1"
port = 8080
enable_cors = true
max_body_size = 10485760      # 10MB
timeout_secs = 30
```

#### 完整配置（含中间件与 SPA 应用）

```toml
[log_config]
level = "debug"
prefix = "my-app"
dir = "./logs"
console_output = true
time_format = "local"

[web_config]
host = "0.0.0.0"
port = 8888
enable_cors = true
max_body_size = 10485760      # 10MB
timeout_secs = 6
static_dir = "./static"
layers = [
    [10, "api_log"],          # API 请求日志
    [100, "compression"],     # 响应压缩
    [10000, "cors"],          # CORS（最外层，最先接收请求）
]

[web_config.spa_apps]
"/admin" = "./static/admin/dist"
"/mood"  = "./static/mood/dist"
```

#### IPv6 配置

```toml
[web_config]
host = "::1"                   # IPv6 localhost
# host = "::"                  # 监听所有 IPv6 接口
port = 8080
enable_cors = true
max_body_size = 10485760
```

### 4. 完整示例

```rust
use tx_di_core::{app, BuildContext};
use tx_di_axum;  // 触发 Web 组件注册
use tx_di_log;   // 触发日志组件注册

app! { AppModule }

#[tokio::main]
async fn main() {
    // 从配置文件加载
    let mut ctx = BuildContext::new(Some("configs/app.toml"));
    ctx.build_and_run().await.expect("启动失败");

    // WebPlugin 已自动启动 Web 服务器
    // 访问 http://127.0.0.1:8080/health 查看健康检查

    // 可以注入 WebConfig 查看配置
    let config = ctx.inject::<tx_di_axum::WebConfig>();
    println!("监听地址: {}", config.address());

    // 保持程序运行
    tokio::signal::ctrl_c().await.ok();
}
```

## 📖 配置项说明

### WebConfig 配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | String | `"127.0.0.1"` | 服务器监听地址，支持 IPv4 和 IPv6 |
| `port` | u16 | `8080` | 服务器监听端口 |
| `enable_cors` | bool | `false` | 是否启用跨域资源共享（CORS） |
| `max_body_size` | usize | `10485760` (10MB) | 最大请求体大小（字节） |
| `static_dir` | String | `"./static"` | 静态文件目录路径 |
| `spa_apps` | Map\<String, String\> | `None` | SPA 应用映射表（路径前缀 → dist 目录） |
| `timeout_secs` | u64 | `30` | 请求超时时间（秒） |
| `layers` | Vec\<(i32, String)\> | `None` | 中间件配置列表（优先级, 名称） |

### 支持的地址格式

**IPv4 地址：**
- `"127.0.0.1"` — 本地回环
- `"0.0.0.0"` — 监听所有网络接口
- `"192.168.1.100"` — 特定 IP

**IPv6 地址：**
- `"::1"` — IPv6 本地回环
- `"::"` — 监听所有 IPv6 接口
- `"2001:db8::1"` — 特定 IPv6 地址

框架会自动处理 IPv6 地址的方括号格式，IPv6 模式下自动启用双栈（同时接受 IPv4 连接）。

## 🧅 中间件系统

### 内置中间件

| 名称 | 说明 | 典型优先级 |
|------|------|------------|
| `cors` | 跨域资源共享，允许浏览器跨域请求 | 10000（最外层） |
| `compression` | 响应压缩（gzip + brotli） | 100 |
| `api_log` | API 请求/响应日志，记录方法、URI、请求体、响应体、耗时 | 10 |
| `trace` | 链路追踪日志，记录请求生命周期事件 | 3 |
| `timeout` | 请求超时控制，超时返回 408 | 5 |

### 中间件优先级与洋葱模型

中间件按优先级排序，**sort 值越小越靠近 Handler（内层），sort 值越大越先接收请求（外层）**：

```text
请求方向 →
┌─ cors (10000) ──────────────────────────────┐
│ ┌─ compression (100) ──────────────────────┐ │
│ │ ┌─ api_log (10) ────────────────────────┐ │ │
│ │ │         ┌─ Handler ──┐                │ │ │
│ │ │         └────────────┘                │ │ │
│ │ └───────────────────────────────────────┘ │ │
│ └───────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
← 响应方向
```

**配置示例：**

```toml
layers = [
    [10, "api_log"],        # 内层：记录业务请求日志
    [100, "compression"],   # 中层：压缩响应
    [10000, "cors"],        # 外层：最先接收请求，处理跨域
]
```

### API 日志中间件（api_log）

内置的 `api_log` 中间件提供详细的请求/响应日志：

**功能特性：**
- 记录请求方法、URI、查询参数、Content-Type
- 记录请求体和响应体（仅 JSON / XML / 文本类型）
- 自动计算请求耗时（自适应 ns / μs / ms 单位）
- 响应状态码非成功时使用 `warn` 级别
- 自动过滤静态资源请求（`/static/*` 及 SPA 路径）

**日志输出示例：**

```text
2026-04-26T14:30:45.123+08:00  INFO REQ  method=POST uri=/api/users query="" content_type=application/json body={"name":"alice"}
2026-04-26T14:30:45.234+08:00  INFO RESP status=200 latency=1.23ms content_type=application/json body={"id":1,"name":"alice"}
```

### 自定义中间件

通过实现 `DynMiddleware` trait 注册自定义中间件：

```rust
use axum::Router;
use tx_di_axum::layers::{DynMiddleware, add_layer};

#[derive(Clone)]
struct AuthLayer;

impl DynMiddleware for AuthLayer {
    fn apply_to_router(&self, router: Router) -> Router {
        router.layer(axum::middleware::from_fn(auth_middleware))
    }

    fn name(&self) -> &str {
        "auth"
    }
}

// 注册到全局中间件（sort=1，最靠近 Handler）
add_layer(AuthLayer, 1);
```

## 🗂 静态文件与 SPA 应用

### 传统静态文件

将文件放置在 `static_dir` 配置的目录下（默认 `./static`），通过 `/static` 路径访问：

```text
./static/
├── logo.svg
├── style.css
└── app.js
```

访问 `http://127.0.0.1:8080/static/logo.svg`，自动支持 gzip/brotli 预压缩文件。

### SPA 应用托管

支持在同一域名下托管多个前端 SPA 应用：

```toml
[web_config.spa_apps]
"/admin" = "./static/admin/dist"    # 管理后台
"/mood"  = "./static/mood/dist"     # 心情记录应用
```

- SPA 应用通过路径前缀区分（如 `/admin`、`/mood`）
- 自动处理前端路由 fallback（返回 `index.html`）
- 支持预压缩的 gzip/brotli 文件
- SPA 路径自动加入日志过滤列表，避免静态资源日志干扰

## 📂 API 端点

### 内置端点

| 路径 | 方法 | 响应类型 | 说明 |
|------|------|----------|------|
| `/health` | GET | `ApiR<FormattedDateTime>` | 健康检查，返回当前时间 |
| `/di` | GET | `ApiR<String>` | 返回 Web 服务器监听地址 |

### 健康检查示例

```bash
curl http://127.0.0.1:8080/health
# {"code":0,"data":"2026-04-26 14:30:45","msg":"success"}
```

## 🔧 高级用法

### 注册自定义路由

使用 `WebPlugin::add_router()` 在应用启动前注册路由：

```rust
use axum::{Router, routing::{get, post}, Json};
use serde_json::json;
use tx_di_axum::WebPlugin;

fn register_api_routes() {
    let api_router = Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users", post(create_user))
        .route("/api/status", get(get_status));

    WebPlugin::add_router(api_router);
}

async fn list_users() -> Json<serde_json::Value> {
    Json(json!({
        "users": [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]
    }))
}
```

### DiComp<T> — 在 Handler 中注入 DI 组件

`DiComp<T>` 是 axum 的请求提取器，可在 Handler 中直接从 DI 容器获取组件：

```rust
use axum::Json;
use tx_di_axum::{DiComp, R};
use tx_di_core::ApiR;

// 在 Handler 中注入 WebConfig
async fn get_config(web_config: DiComp<WebConfig>) -> R<String> {
    ApiR::success(web_config.address()).into()
}

// 注入任意 DI 组件
async fn get_db_status(db_pool: DiComp<DbPool>) -> R<String> {
    ApiR::success("database connected".to_string()).into()
}
```

**工作原理：**
- `WebPlugin` 在启动时将 `App` 实例注入到请求扩展中
- `DiComp<T>` 通过 `FromRequestParts` 从请求扩展中获取 `App`
- 再调用 `App::try_inject::<T>()` 从 DI 容器获取组件
- 组件不存在时返回 `WebErr::IE` 错误响应

### R\<T\> — 统一响应封装

`R<T>` 是对 `ApiR<T>` 的 axum 响应封装，自动序列化为 JSON：

```rust
use tx_di_axum::R;
use tx_di_core::ApiR;

// 成功响应
async fn success() -> R<String> {
    ApiR::success("hello".to_string()).into()
}

// 带数据响应
async fn with_data() -> R<User> {
    ApiR::success(User { id: 1, name: "Alice".into() }).into()
}
```

### WebErr — 错误处理

`WebErr` 实现 `IntoResponse`，自动将错误转换为 HTTP 响应：

```rust
use tx_di_axum::e::WebErr;
use tx_di_core::IE;

// IE（业务错误）→ 200 + ApiRes 错误信息
// Other（系统错误）→ 500 + ApiRes 失败信息
async fn handler() -> Result<R<String>, WebErr> {
    // 业务错误
    // return Err(WebErr::IE(IE::Other("用户不存在".into())));
    
    Ok(ApiR::success("ok".to_string()).into())
}
```

### RequestPartsExt — 请求扩展

在自定义中间件中通过 `RequestPartsExt` 访问 DI 容器：

```rust
use axum::http::request::Parts;
use tx_di_axum::bound::RequestPartsExt;

fn my_middleware(parts: &Parts) {
    // 获取 App 实例
    let app_status = parts.app_status();
    
    // 从 DI 容器获取组件
    let config = parts.get_comp::<WebConfig>().unwrap();
}
```

### 访问全局配置对象

```rust
use tx_di_core::AppAllConfig;

let global_config = ctx.inject::<AppAllConfig>();

// 读取 web 配置
if let Some(host) = global_config.get::<String>("web_config.host") {
    println!("监听地址: {}", host);
}

// 带默认值的读取
let max_size = global_config.get_or_default("web_config.max_body_size", 5242880usize);
```

## 🏗 架构设计

### 组件初始化顺序

| 组件 | init_sort | 说明 |
|------|-----------|------|
| `LogConfig` | `i32::MIN` | 最先初始化，确保日志系统就绪 |
| `LogPlugins` | `i32::MIN` | 紧随配置初始化 |
| `WebConfig` | `i32::MAX` | 配置类，优先于插件 |
| `WebPlugin` | `i32::MAX` | 最后初始化，确保所有路由和中间件已注册 |

### 请求处理流程

```text
                    请求进入
                       │
          ┌────────────▼────────────┐
          │    CorsLayer (10000)     │ ← 最外层：处理跨域
          └────────────┬────────────┘
          ┌────────────▼────────────┐
          │  CompressionLayer (100)  │ ← 压缩响应
          └────────────┬────────────┘
          ┌────────────▼────────────┐
          │    ApiLogLayer (10)      │ ← 记录请求/响应日志
          └────────────┬────────────┘
          ┌────────────▼────────────┐
          │   AppStatus Injection    │ ← 注入 App 到请求扩展
          └────────────┬────────────┘
          ┌────────────▼────────────┐
          │        Handler          │ ← 业务处理
          │   (支持 DiComp<T> 提取) │
          └────────────┬────────────┘
                       │
                    响应返回
```

### 静态文件路径过滤

日志系统采用「先收集、后冻结」的策略过滤静态文件请求：

1. **初始化阶段**：通过 `RwLock` 收集所有静态文件路径前缀（`/static` + SPA 路径）
2. **启动阶段**：调用 `freeze_static_path_prefixes()` 冻结为不可变 `Vec`
3. **运行阶段**：`should_filter_path()` 零锁访问，实现高性能过滤

## ⚠️ 注意事项

### 1. 必须导入插件

```rust
// ✅ 正确
use tx_di_axum;

// ❌ 错误：仅在 Cargo.toml 中声明依赖，但代码中未使用
// 结果：WebPlugin 不会注册，Web 服务器不会启动
```

### 2. 初始化顺序

`WebPlugin` 的初始化排序设置为 `i32::MAX`，在日志插件（`i32::MIN`）之后初始化，确保日志系统已经就绪。如果需要在 Web 初始化前注册路由或中间件，请在 `WebPlugin::init_sort()` 之前的组件中完成。

### 3. 端口占用

如果配置的端口已被占用，服务器会返回错误并在日志中记录。请确保端口可用或更改配置。

### 4. 异步运行时

`WebPlugin` 的服务器启动在 `async_init` 阶段执行，需要在 tokio 异步运行时中运行。确保主函数使用了 `#[tokio::main]` 或在 tokio runtime 中调用。

```rust
#[tokio::main]
async fn main() {
    let mut ctx = BuildContext::new(Some("configs/app.toml"));
    ctx.build_and_run().await.expect("启动失败");
}
```

### 5. 保持程序运行

Web 服务器在后台异步运行，需要保持主线程不退出：

```rust
// 推荐：等待 Ctrl+C
tokio::signal::ctrl_c().await.ok();
```

### 6. IPv6 双栈

当配置 IPv6 地址时，框架自动设置 `IPV6_V6ONLY=false`，使 IPv6 socket 也能接受 IPv4 连接。无需额外配置。

## 🧪 测试

```bash
# 运行插件测试
cd plugins/tx_di_axum
cargo test

# 运行完整示例
cd di-example
cargo run
```

### 测试覆盖

- ✅ IPv4 地址格式验证
- ✅ IPv6 地址格式验证（自动添加方括号）
- ✅ IPv6 通配符地址 `::`
- ✅ IPv6 完整地址格式
- ✅ 中间件优先级排序
- ✅ 静态文件路径过滤
- ✅ DiComp\<T\> 组件注入

## 📦 依赖说明

| 依赖 | 说明 |
|------|------|
| `axum` | Web 框架 |
| `tokio` | 异步运行时 |
| `tower` | 中间件抽象层 |
| `tower-http` | HTTP 中间件实现（CORS/压缩/追踪/超时/静态文件） |
| `socket2` | 底层 socket 配置（IPv6 双栈/地址重用） |
| `tx-di-core` | tx_di 核心框架 |
| `tx_di_log` | 日志插件（推荐配合使用） |
| `serde` | 配置反序列化 |
| `tracing` | 结构化日志 |
| `anyhow` | 错误处理 |
| `thiserror` | 错误类型定义 |

## 🔗 相关链接

- [tx_di 主仓库](https://github.com/txtxtxtx/tx_di.git)
- [axum 文档](https://docs.rs/axum)
- [tower-http 文档](https://docs.rs/tower-http)
- [tokio 文档](https://docs.rs/tokio)

## 📄 许可证

与 tx_di 主项目保持一致。
