# tx_di 插件开发规范与分析报告

## 一、插件架构概览

基于 `tx_di` 依赖注入框架的插件系统，采用编译期元数据收集、运行期自动注入的设计，零反射、零运行时扫描。

### 核心特性
- **编译期注册**：通过 `linkme` 在编译期收集组件元数据
- **自动注入**：基于拓扑排序的依赖注入，支持 Singleton 和 Prototype 作用域
- **配置驱动**：TOML 配置文件自动反序列化
- **生命周期管理**：提供 `CompInit` trait 控制组件初始化顺序

---

## 二、标准插件结构

### 2.1 目录结构
```
tx_di_<plugin_name>/
├── Cargo.toml          # 包配置
├── README.md            # 插件文档
└── src/
    ├── lib.rs           # 模块导出
    ├── config.rs        # 配置结构体
    └── comp.rs          # 组件实现
```

### 2.2 Cargo.toml 规范

```toml
[package]
name = "tx_di_<plugin_name>"
edition.workspace = true
version.workspace = true
authors.workspace = true
license.workspace = true
readme = "README.md"
repository.workspace = true

[features]
default = ["default-feature"]
# 可选功能特性
special_feature = ["dep:some-crate"]

[dependencies]
# 核心依赖（必须）
tx-di-core.workspace = true
linkme.workspace = true

# 工作区依赖（推荐）
tx_error = { workspace = true }
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tokio.workspace = true

# 可选依赖
some-crate = { workspace = true, optional = true }
```

**关键点**：
- 使用 `workspace = true` 继承工作区配置
- 可选功能通过 `features` 控制
- 可选依赖使用 `optional = true`

---

## 三、核心组件开发规范

### 3.1 配置结构体（config.rs）

**用途**：定义插件配置，支持从 TOML 文件自动加载

**标准模式**：
```rust
use serde::Deserialize;
use tx_di_core::{tx_comp, CompInit, InnerContext, RIE};

/// 配置结构体
///
/// 支持从 TOML 配置文件自动反序列化
///
/// # 配置文件示例
/// ```toml
/// [<config_section>]
/// field1 = "value1"
/// field2 = 8080
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf, init)]  // ← 关键：启用配置注入 + 自定义初始化
pub struct MyConfig {
    /// 字段说明
    #[serde(default = "default_field1")]
    pub field1: String,

    /// 数值字段
    #[serde(default = "default_field2")]
    pub field2: u16,
}

/// 必须实现 Default trait
impl Default for MyConfig {
    fn default() -> Self {
        Self {
            field1: default_field1(),
            field2: default_field2(),
        }
    }
}

/// 可选：实现 CompInit 用于配置验证或预处理
impl CompInit for MyConfig {
    /// 在依赖注入完成后、应用启动前调用
    fn inner_init(&mut self, _: &InnerContext) -> RIE<()> {
        // 配置验证或预处理逻辑
        Ok(())
    }

    /// 控制初始化顺序（值越小越先执行）
    /// 建议使用 i32::MIN 或 i32::MAX 明确优先级
    fn init_sort() -> i32 {
        i32::MIN  // 需要早初始化的配置
        // 或 i32::MAX  // 需要晚初始化的配置
    }
}

// 提供默认值函数
fn default_field1() -> String {
    "default_value".to_string()
}

fn default_field2() -> u16 {
    8080
}
```

**关键点**：
- `#[tx_comp(conf, init)]`：启用配置注入 + 自定义初始化
- `#[serde(default = "fn_name")]`：提供默认值
- 必须实现 `Default` trait
- 可选实现 `CompInit` 进行配置验证

---

### 3.2 组件实现（comp.rs）

**用途**：实现插件核心功能，管理生命周期

**标准模式**：
```rust
use std::sync::Arc;
use tx_di_core::{tx_comp, CompInit, App, RIE, async_method};
use tokio_util::sync::CancellationToken;

/// 插件组件
///
/// 实现核心业务逻辑，支持依赖注入和生命周期管理
#[tx_comp(init)]  // ← 关键：启用自定义初始化
pub struct MyPlugin {
    /// 依赖注入的配置
    pub config: Arc<MyConfig>,

    /// 内部状态（使用 OnceLock 或 RwLock）
    #[tx_cst(OnceLock::new())]
    pub state: OnceLock<SomeState>,
}

impl CompInit for MyPlugin {
    // ===== 同步初始化（构建阶段） =====
    /// 在组件构建后立即调用（BuildContext 阶段）
    /// 用于验证配置、初始化内部状态
    fn inner_init(&mut self, ctx: &InnerContext) -> RIE<()> {
        println!("MyPlugin: inner_init");
        // 验证配置
        // 初始化内部状态
        Ok(())
    }

    // ===== 同步初始化（应用阶段） =====
    /// 在应用启动时调用（所有组件构建完成后）
    /// 用于需要访问其他组件的初始化逻辑
    fn init(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
        println!("MyPlugin: init");
        // 访问其他组件
        // let other_comp = ctx.inject::<OtherComp>();
        Ok(())
    }

    // ===== 异步初始化 =====
    /// 在应用启动时异步调用
    /// 用于需要异步操作的初始化（如数据库连接）
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            println!("MyPlugin: async_init");
            // 异步初始化逻辑
            Ok(())
        }
    );

    // ===== 异步运行（长期任务） =====
    /// 在应用启动后异步运行
    /// 用于启动后台任务（如 HTTP 服务器、消息消费者）
    async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            println!("MyPlugin: async_run");
            // 启动长期运行的任务
            // token.cancelled().await;  // 等待关闭信号
            Ok(())
        }
    );

    /// 控制初始化顺序
    /// - i32::MIN: 最早初始化（如日志、配置）
    /// - 10000: 默认顺序
    /// - i32::MAX: 最晚初始化（如 Web 服务器）
    fn init_sort() -> i32 {
        i32::MAX  // 通常在最后启动
    }
}

impl MyPlugin {
    /// 公共方法
    pub fn some_method(&self) -> RIE<()> {
        Ok(())
    }
}
```

**生命周期钩子说明**：
1. `inner_init()`: 组件构建时同步调用，用于验证和预处理
2. `init()`: 应用启动时同步调用，可访问其他组件
3. `async_init_impl()`: 应用启动时异步调用，用于异步初始化
4. `async_run_impl()`: 应用运行后异步调用，用于启动后台任务

**初始化顺序建议**：
- `i32::MIN`: 日志、配置等基础组件
- `10000`: 普通业务组件
- `i32::MAX`: Web 服务器等需要等待其他组件就绪的组件

---

### 3.3 模块导出（lib.rs）

**用途**：声明模块、重导出公共 API

**标准模式**：
```rust
// 私有模块
mod config;
mod comp;

// 公共模块（如有）
pub mod e;      // 错误定义
pub mod err;    // 错误码
pub mod utils;  // 工具函数

// 重导出
pub use config::*;
pub use comp::*;

// 条件编译的重导出
#[cfg(feature = "special-feature")]
pub use some_crate;
```

---

## 四、常见集成模式

### 4.1 集成 Axum Web 服务器

```rust
use axum::{Router, routing::get};
use tx_di_axum::{WebPlugin, Router as TxRouter};

// 定义路由
fn setup_routes() -> TxRouter {
    TxRouter::new()
        .route("/api/resource", get(handler))
}

// 在 async_init_impl 中注册路由
async_method!(
    fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
        let router = setup_routes();
        WebPlugin::add_router(router);
        Ok(())
    }
);
```

### 4.2 添加中间件

```rust
use tower::{Layer, ServiceBuilder};
use tx_di_axum::layers::{LayerRegistry, add_layer};

// 定义中间件
struct MyLayer;

impl<S> Layer<S> for MyLayer {
    type Service = MyMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyMiddleware { inner }
    }
}

// 注册中间件
fn register_middleware() {
    add_layer(100, MyLayer);  // 100 是优先级（越大越外层）
}
```

### 4.3 使用 Tracing 日志

```rust
use tracing::{info, debug, error, warn};

// 在组件方法中使用
fn some_method(&self) {
    info!("信息日志");
    debug!("调试日志");
    warn!("警告日志");
    error!("错误日志");
}
```

---

## 五、配置文件规范

### 5.1 配置文件位置
```
configs/
└── di-config.toml    # 主配置文件
```

### 5.2 配置段命名
配置段名称应与配置结构体名称匹配（snake_case）：
- `MyConfig` → `[my_config]`
- `WebConfig` → `[web_config]`
- `LogConfig` → `[log_config]`

### 5.3 配置示例
```toml
[my_config]
field1 = "value1"
field2 = 8080

[other_config]
# ...
```

---

## 六、最佳实践

### 6.1 错误处理
- 使用 `anyhow::Result` 或自定义错误类型
- 在 `inner_init` 中验证配置
- 使用 `tracing` 记录错误信息

### 6.2 依赖注入
- 使用 `Arc<T>` 注入依赖
- 在 `init()` 或 `async_init_impl()` 中获取依赖
- 避免循环依赖

### 6.3 异步编程
- 使用 `async_method!` 宏简化异步代码
- 在 `async_run_impl()` 中启动长期任务
- 使用 `CancellationToken` 处理优雅关闭

### 6.4 测试
- 在 `lib.rs` 中添加 `#[cfg(test)] mod tests;`
- 使用 `BuildContext::new()` 创建测试上下文
- 使用 `ctx.build().ins_run().await` 启动应用

---

## 七、插件生成 Skill 设计

基于以上分析，设计用于生成插件的 Skill：

### 7.1 Skill 输入
用户描述需求，例如：
- "我需要一个 Redis 缓存插件"
- "创建一个 SMTP 邮件发送插件"
- "集成 Kafka 消息队列"

### 7.2 Skill 输出
完整的插件代码，包括：
1. `Cargo.toml` - 包配置
2. `src/lib.rs` - 模块声明
3. `src/config.rs` - 配置结构体
4. `src/comp.rs` - 组件实现
5. `README.md` - 使用文档

### 7.3 生成逻辑
1. **解析用户需求**：识别功能、依赖、配置项
2. **生成配置结构体**：根据需求生成 `config.rs`
3. **生成组件实现**：根据需求生成 `comp.rs`，实现必要的生命周期钩子
4. **生成 Cargo.toml**：根据依赖生成包配置
5. **生成文档**：生成 README.md 说明使用方法

### 7.4 模板变量
- `{{plugin_name}}` - 插件名称
- `{{description}}` - 功能描述
- `{{dependencies}}` - 依赖列表
- `{{config_fields}}` - 配置字段
- `{{init_logic}}` - 初始化逻辑
- `{{run_logic}}` - 运行逻辑

---

## 八、总结

本报告详细分析了 `tx_di` 插件的：
- 标准目录结构和文件组织
- `Cargo.toml` 配置规范
- 配置结构体和组件实现的代码模式
- 生命周期管理和初始化顺序
- 常见集成模式（Axum、中间件、日志）
- 配置文件规范
- 最佳实践和错误处理

基于这些规范，可以创建一个 Skill 来根据用户需求自动生成符合规范的插件代码。

---

**下一步**：确认报告内容无误后，我将创建 Skill 实现插件自动生成功能。
