# tx_di_log — 日志插件使用文档

基于 `tracing` + `tracing-subscriber` + `tracing-appender` 的日志插件，是 `tx-di` 应用的首选基础插件。

## 用途

- 在 App 构建阶段（`init_sort = i32::MIN`，最先执行）初始化**全局 tracing 订阅者**。
- 提供**按天滚动的文件输出**（含线程 ID、文件名、行号、本地/UTC 时间）以及可选的控制台彩色输出。
- 支持按模块粒度覆盖日志级别（`modules`）。
- 设置 panic hook：**仅记录日志，不强制退出进程**（避免破坏测试/嵌入式场景）。

> 务必最先引入本插件，否则其他插件的日志无法落盘。

## 启用

`Cargo.toml`：

```toml
tx_di_log = { path = "plugins/tx_di_log" }
```

代码里**必须** `use tx_di_log;`（空导入即可），否则该 crate 被链接器优化掉、`LogPlugins` 不会被注册：

```rust
use tx_di_core::{BuildContext, app};
use tx_di_log;          // 关键：触发 linkme 注册

app! { AppModule }

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let app = ctx.build()?;
    let app = app.ins_run().await?;
    // 或直接在构建前注入以触发初始化：
    // let _ = ctx.inject::<tx_di_log::LogPlugins>();
    tracing::info!("应用启动");
    Ok(())
}
```

## 配置

TOML 节名为 `[log_config]`：

```toml
[log_config]
level = "info"                  # 全局级别: off/error/warn/info/debug/trace
console_output = true           # 是否输出到控制台（默认 true）
time_format = "local"           # utc / local
retention_days = 90             # 日志文件保留天数
prefix = "tx_di"                # 日志文件名前缀
dir = "./logs"                  # 日志文件目录

[log_config.modules]            # 模块级别日志覆盖（可选）
"my_app::database" = "debug"
"third_party_lib" = "warn"
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `level` | `log::LevelFilter` | `info` |
| `modules` | `HashMap<String, LevelFilter>` | `{}` |
| `format` | `String` | `""` |
| `dir` | `PathBuf` | 可执行文件同级 `./logs` |
| `retention_days` | `usize` | `90` |
| `console_output` | `bool` | `true` |
| `prefix` | `String` | `"tx_di"` |
| `time_format` | `TimeFormat`（`utc`/`local`） | `local` |
| `time_format_str` | `String` | `"[hour]:[minute]:[second].[subsecond digits:3]"` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `LogPlugins` | `init`, `init_sort = i32::MIN` | 持有 `Arc<LogConfig>`，在 `init` 回调中完成 tracing 初始化 |
| `LogConfig` | `conf = "log"` | 配置载体，可 `ctx.inject::<LogConfig>()` 读取 |

## 注意事项

1. **必须 `use tx_di_log;`**：仅声明依赖不导入，组件不注册，日志订阅者不会初始化。
2. **panic hook 不退出进程**：`LogPlugins::init` 设置的 panic hook 只记录 `error!`，不会 `std::process::exit`。如需进程级退出策略请自行处理。
3. **初始化时机**：`LogPlugins` 的 `init_sort = i32::MIN`，保证最早初始化，因此其他插件（其 `init`/`app_async_init` 中产生的日志）都能正常落盘。
4. 日志通过标准 `tracing`/`log` 宏（`info!`/`debug!`/`warn!`/`error!`）输出。
