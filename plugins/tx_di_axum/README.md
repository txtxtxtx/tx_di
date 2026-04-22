# tx_di_axum

基于 axum 的 web 服务器插件，为 tx_di 依赖注入框架提供 web 服务支持。

## ✨ 特性

- 🚀 **高性能**：基于 axum 和 tokio 的异步 web 服务器
- ⚙️ **TOML 配置加载**：通过配置文件灵活定制 web 服务器参数
- 🔧 **依赖注入集成**：与 tx_di 框架无缝集成
- 🏥 **健康检查端点**：内置 `/health` 路由用于监控
- 🌐 **CORS 支持**：可选的跨域资源共享配置
- 📦 **请求体限制**：可配置的最大请求体大小

## 🚀 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
tx_di_axum = { path = "../plugins/tx_di_axum" }
tx-di-core = { path = "../tx-di-core" }
linkme = "0.3"  # 或使用你项目中的版本
```

### 2. ⚠️ 导入插件（必须步骤）

```rust
use tx_di_core::{app, BuildContext};
use tx_di_axum;  // ← 必须导入以触发 WebPlugin 组件注册

app! { AppModule }
```

**为什么需要这一步？**

tx_di 使用 `linkme` 进行编译期静态注册。如果代码中没有引用 `tx_di_axum`，Rust 链接器会优化掉这个 crate，导致 `WebPlugin` 组件无法注册到 DI 容器。

通过 `use tx_di_axum;` 确保 crate 被链接，触发组件自动注册。

### 3. 创建配置文件

创建 `configs/web.toml`：

#### IPv4 配置

```toml
[web_config]
host = "127.0.0.1"          # 监听地址
port = 8080                  # 监听端口
enable_cors = true           # 是否启用 CORS
max_body_size = 10485760     # 最大请求体大小（字节）
```

#### IPv6 配置

```toml
[web_config]
host = "::1"                 # IPv6 localhost
# host = "::"               # 监听所有 IPv6 接口
port = 8080
enable_cors = true
max_body_size = 10485760
```

### 4. 完整示例

```rust
use std::path::PathBuf;
use tx_di_core::{app, BuildContext};
use tx_di_axum;
use tx_di_log;  // 推荐配合日志插件使用
use log::info;

app! { AppModule }

#[tokio::main]
async fn main() {
    // 方式 1：从配置文件加载
    let mut ctx = BuildContext::new(Some("configs/web.toml"));
    
    // 方式 2：自动扫描（使用默认配置）
    // let mut ctx = BuildContext::new::<PathBuf>(None);
    
    ctx.run().await;
    
    // WebPlugin 已自动启动 web 服务器
    info!("Web 服务器已在后台启动");
    
    // 可以注入 WebConfig 查看配置
    let config = ctx.inject::<tx_di_axum::WebConfig>();
    println!("监听地址: {}", config.address());
    
    // 保持程序运行
    tokio::signal::ctrl_c().await.ok();
}
```

## 📖 配置项说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | String | `"127.0.0.1"` | 服务器监听地址，支持 IPv4 和 IPv6 |
| `port` | u16 | `8080` | 服务器监听端口 |
| `enable_cors` | bool | `false` | 是否启用跨域资源共享（CORS） |
| `max_body_size` | usize | `10485760` (10MB) | 最大请求体大小（字节） |

### 支持的地址格式

**IPv4 地址：**
- `"127.0.0.1"` - 本地回环
- `"0.0.0.0"` - 监听所有网络接口
- `"192.168.1.100"` - 特定 IP

**IPv6 地址：**
- `"::1"` - IPv6 本地回环
- `"::"` - 监听所有 IPv6 接口
- `"2001:db8::1"` - 特定 IPv6 地址

框架会自动处理 IPv6 地址的方括号格式。

## 📂 API 端点

### 健康检查

- **路径**: `/health`
- **方法**: GET
- **响应**: `OK` (纯文本)
- **用途**: 负载均衡器或监控系统的健康检查

```bash
curl http://127.0.0.1:8080/health
# 输出: OK
```

## 🔧 高级用法

### 扩展路由器添加自定义路由

你可以通过依赖注入获取 `WebPlugin` 实例，然后添加自定义路由：

```rust
use axum::{Router, routing::get, Json};
use serde_json::json;
use tx_di_core::{tx_comp, BuildContext};
use tx_di_axum::WebPlugin;
use std::sync::Arc;

#[derive(Clone)]
#[tx_comp(Singleton)]
pub struct ApiRoutes {
    pub web_plugin: Arc<WebPlugin>,
}

impl ApiRoutes {
    pub fn register_routes(&mut self) {
        // 添加自定义路由
        self.web_plugin.router = self.web_plugin.router.clone()
            .route("/api/users", get(list_users))
            .route("/api/status", get(get_status));
    }
}

async fn list_users() -> Json<serde_json::Value> {
    Json(json!({
        "users": [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]
    }))
}

async fn get_status() -> Json<serde_json::Value> {
    Json(json!({
        "status": "running",
        "version": "1.0.0"
    }))
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

if let Some(port) = global_config.get::<u16>("web_config.port") {
    println!("监听端口: {}", port);
}

// 带默认值的读取
let max_size = global_config.get_or_default("web_config.max_body_size", 5242880usize);
```

### 与其他插件配合使用

```rust
use tx_di_core::{app, BuildContext};
use tx_di_log;
use tx_di_axum;

// 导入所有插件以触发注册
use tx_di_log;
use tx_di_axum;

app! { AppModule }

#[tokio::main]
async fn main() {
    let mut ctx = BuildContext::new(Some("configs/app.toml"));
    ctx.run().await;
    
    // 所有插件已就绪
    info!("应用启动完成");
    info!("Web 服务器正在运行");
    
    // 保持程序运行
    tokio::signal::ctrl_c().await.ok();
}
```

## ⚠️ 注意事项

### 1. 必须导入插件

```rust
// ✅ 正确
use tx_di_axum;

// ❌ 错误：仅在 Cargo.toml 中声明依赖，但代码中未使用
// 结果：WebPlugin 不会注册，web 服务器不会启动
```

### 2. 初始化顺序

`WebPlugin` 的初始化顺序设置为 `i32::MIN + 100`，在日志插件之后初始化，确保日志系统已经就绪。

### 3. 端口占用

如果配置的端口已被占用，服务器会返回错误并在日志中记录。请确保端口可用或更改配置。

### 4. 异步运行时

`WebPlugin` 需要在 tokio 异步运行时中运行。确保你的主函数使用了 `#[tokio::main]` 或在 tokio runtime 中调用。

```rust
#[tokio::main]
async fn main() {
    let mut ctx = BuildContext::new(Some("configs/web.toml"));
    ctx.run().await;
    
    // Web 服务器在后台运行
    tokio::signal::ctrl_c().await.ok();
}
```

### 5. 保持程序运行

Web 服务器在后台异步运行，需要保持主线程不退出：

```rust
// 方式 1：等待 Ctrl+C
tokio::signal::ctrl_c().await.ok();

// 方式 2：使用 thread::park()
std::thread::park();

// 方式 3：使用 sleep（不推荐）
tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
```

## 🧪 测试

```bash
# 运行插件测试
cd plugins/tx_di_axum
cargo test

# 运行示例（IPv4）
cd di-example
cargo run

# 使用 IPv6 配置
cargo run -- --config configs/web-ipv6.toml
```

### 测试覆盖

- ✅ IPv4 地址格式验证
- ✅ IPv6 地址格式验证（自动添加方括号）
- ✅ IPv6 通配符地址 `::`
- ✅ IPv6 完整地址格式

## 📦 依赖说明

- `axum`: Web 框架
- `tokio`: 异步运行时
- `tx-di-core`: tx_di 核心框架
- `tracing`: 结构化日志
- `serde`: 配置反序列化
- `anyhow`: 错误处理

## 🔗 相关链接

- [tx_di 主仓库](https://github.com/txtxtxtx/tx_di.git)
- [axum 文档](https://docs.rs/axum)
- [tokio 文档](https://docs.rs/tokio)

## 📄 许可证

与 tx_di 主项目保持一致。
