# tx_di_log

tx_di 框架的日志插件，基于 `tracing` + `tracing-subscriber` 生态构建，提供强大的日志记录能力。

## ✨ 特性

- 📁 **文件滚动**：按天自动滚动，支持保留天数配置
- 🎨 **控制台彩色输出**：开发时友好的彩色日志格式
- 🔧 **模块级别过滤**：支持不同模块设置不同日志级别
- ⚙️ **TOML 配置加载**：通过配置文件灵活定制
- 🕐 **时间格式支持**：UTC 和本地时间两种模式
- 🛡️ **Panic 捕获**：自动记录程序异常终止信息

## 🚀 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
tx_di_log = { path = "../plugins/tx_di_log" }
tx-di-core = { path = "../tx-di-core" }
linkme = "0.3"  # 或使用你项目中的版本
```

### 2. ⚠️ 导入插件（必须步骤）

```rust
use tx_di_core::{app, BuildContext};
use tx_di_log;  // ← 必须导入以触发 LogPlugins 组件注册

app! { AppModule }
```

**为什么需要这一步？**

tx_di 使用 `linkme` 进行编译期静态注册。如果代码中没有引用 `tx_di_log`，Rust 链接器会优化掉这个 crate，导致 `LogPlugins` 组件无法注册到 DI 容器。

通过 `use tx_di_log;` 确保 crate 被链接，触发组件自动注册。

### 3. 创建配置文件

创建 `configs/log.toml`：

```toml
[log_config]
level = "info"                    # 日志级别：off/error/warn/info/debug/trace
dir = "./logs"                    # 日志文件目录
console_output = true             # 是否输出到控制台
time_format = "local"             # 时间格式：utc/local
retention_days = 90               # 日志保留天数
prefix = "my-app"                 # 日志文件前缀

# 可选：模块级别的日志覆盖
[log_config.modules]
"my_crate::verbose_module" = "debug"
"another_crate" = "warn"
```

### 4. 完整示例

```rust
use std::path::PathBuf;
use tx_di_core::{app, BuildContext};
use tx_di_log;  // 触发组件注册
use log::{info, debug, warn, error};

app! { AppModule }

#[tokio::main]
async fn main() {
    // 方式 1：从配置文件加载
    let mut ctx = BuildContext::new(Some("configs/log.toml"));
    
    // 方式 2：自动扫描（使用默认配置）
    // let mut ctx = BuildContext::new::<PathBuf>(None);
    
    ctx.run().await;
    
    // LogPlugins 已自动初始化日志系统
    info!("应用启动");
    debug!("调试信息");
    warn!("警告信息");
    error!("错误信息");
    
    // 可以注入 LogConfig 查看配置
    let config = ctx.inject::<tx_di_log::LogConfig>();
    println!("日志级别: {:?}", config.level);
    println!("日志目录: {:?}", config.dir);
}
```

## 📖 配置项说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `level` | String | `"info"` | 全局日志级别：`off`/`error`/`warn`/`info`/`debug`/`trace` |
| `dir` | Path | `./logs` | 日志文件存储目录 |
| `console_output` | bool | `false` | 是否同时输出到控制台 |
| `time_format` | String | `"utc"` | 时间格式：`utc`（协调世界时）或 `local`（本地时区） |
| `retention_days` | u64 | `90` | 日志文件保留天数，超期自动删除 |
| `prefix` | String | `"tx_di"` | 日志文件名前缀，如 `my-app-2026-04-22.log` |
| `modules` | Map | `{}` | 模块级别的日志级别覆盖配置 |

### 日志级别说明

- `off`: 完全禁用日志
- `error`: 仅记录错误
- `warn`: 记录警告和错误
- `info`: 记录信息、警告和错误（推荐生产环境）
- `debug`: 记录调试信息及更高级别（推荐开发环境）
- `trace`: 记录所有日志，包括最详细的跟踪信息

### 时间格式说明

- `utc`: UTC 时间，格式如 `2026-04-22T06:30:45.123456789Z`
- `local`: 本地时间，格式如 `2026-04-22T14:30:45.123456789+08:00`

## 📂 日志文件组织

```
logs/
├── my-app-2026-04-20.log
├── my-app-2026-04-21.log
└── my-app-2026-04-22.log  # 当前日志文件
```

日志文件按天滚动，超过 `retention_days` 配置的旧文件会自动清理。

## 🔧 高级用法

### 模块级别日志控制

在配置文件中为不同模块设置不同的日志级别：

```toml
[log_config]
level = "info"

[log_config.modules]
"my_app::database" = "debug"      # 数据库模块显示调试信息
"my_app::api" = "trace"           # API 模块显示详细信息
"third_party_lib" = "warn"        # 第三方库仅显示警告
```

### 访问全局配置对象

```rust
use tx_di_core::AppAllConfig;

let global_config = ctx.inject::<AppAllConfig>();

// 读取日志配置
if let Some(level) = global_config.get::<String>("log_config.level") {
    println!("日志级别: {}", level);
}

// 带默认值的读取
let retention = global_config.get_or_default("log_config.retention_days", 30u64);
```

### 与其他插件配合使用

```rust
use tx_di_core::{app, BuildContext};
use tx_di_log;
use tx_di_axum;  // 假设还有其他插件

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
}
```

## 📝 日志输出示例

### 控制台输出（彩色）

```
2026-04-22T14:30:45.123456789+08:00  INFO [main] my_app: 应用启动
2026-04-22T14:30:45.234567890+08:00 DEBUG [tokio-runtime-worker-1] my_app::db: 数据库连接池初始化完成
2026-04-22T14:30:45.345678901+08:00  WARN [main] my_app::config: 配置文件不存在，使用默认值
```

### 文件输出

```
2026-04-22T06:30:45.123456789Z  INFO ThreadId(1) [main] my_app src/main.rs:42: 应用启动
2026-04-22T06:30:45.234567890Z DEBUG ThreadId(3) [tokio-runtime-worker-1] my_app::db src/db.rs:128: 数据库连接池初始化完成
```

## ⚠️ 注意事项

### 1. 必须导入插件

```rust
// ✅ 正确
use tx_di_log;

// ❌ 错误：仅在 Cargo.toml 中声明依赖，但代码中未使用
// 结果：LogPlugins 不会注册，日志系统不会初始化
```

### 2. 避免重复初始化

`LogPlugins` 使用 `OnceLock` 保护全局守卫，重复初始化会导致错误：

```rust
// 不要多次调用 ctx.run()
let mut ctx = BuildContext::new(Some("configs/log.toml"));
ctx.run().await;  // 第一次：成功
// ctx.run().await;  // 第二次：会 panic
```

### 3. 日志目录权限

确保应用有权限在配置的 `dir` 目录下创建文件和子目录。

### 4. 异步环境

`LogPlugins` 的初始化是同步的，可以在 `run()` 之前或之后安全使用日志宏：

```rust
let mut ctx = BuildContext::new(Some("configs/log.toml"));

// 此时日志尚未初始化
println!("使用 println");

ctx.run().await;

// 此时日志已初始化
info!("使用 tracing 日志");
```

## 🧪 测试

```bash
# 运行插件测试
cd plugins/tx_di_log
cargo test

# 运行示例
cd examples
cargo run --bin log_example
```

## 📦 依赖说明

- `tracing`: 结构化日志框架
- `tracing-subscriber`: 日志订阅者实现
- `tracing-appender`: 日志文件滚动支持
- `tx-di-core`: tx_di 核心框架
- `serde`: 配置反序列化
- `time`: 时间格式化

## 🔗 相关链接

- [tx_di 主仓库](https://github.com/txtxtxtx/tx_di.git)
- [tracing 文档](https://docs.rs/tracing)
- [tracing-subscriber 文档](https://docs.rs/tracing-subscriber)

## 📄 许可证

与 tx_di 主项目保持一致。
