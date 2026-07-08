# tx_di_axum — Web 服务插件使用文档

基于 `axum` + `tokio` 的 HTTP 服务插件，为 `tx-di` 框架提供高性能 Web 能力。

## 用途

- TOML 配置驱动，自动绑定 TCP 端口并启动 `axum::serve`（带优雅停机）。
- 内置中间件洋葱模型：`api_log` / `cors` / `trace` / `timeout` / `compression`。
- 健康检查 `/health`、静态文件 `/static`、SPA 托管、`DiComp<T>` 提取器在 handler 中注入 DI 组件。
- `api-doc` feature（默认开启）下自动生成 `/docs`、`/api-docs/openapi.json` OpenAPI 文档。

## 启用

`Cargo.toml`：

```toml
tx_di_axum = { path = "plugins/tx_di_axum" }
tx_di_log  = { path = "plugins/tx_di_log" }   # 推荐：日志插件
```

**必须 `use tx_di_axum;`**（空导入），否则 `WebPlugin`/`WebConfig` 不会被 linkme 注册，Web 服务器不启动。

## 配置

TOML 节名为 `[web_config]`：

```toml
[web_config]
host = "0.0.0.0"
port = 8888
enable_cors = true
max_body_size = 10485760
timeout_secs = 6
static_dir = "./static"
layers = [
    [10, "api_log"],
    [100, "compression"],
    [10000, "cors"],
]

[web_config.spa_apps]
"/admin" = "./static/admin/dist"
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `host` | `String` | `"127.0.0.1"` |
| `port` | `u16` | `8080` |
| `enable_cors` | `bool` | `false` |
| `max_body_size` | `usize` | `10485760`（10MB） |
| `static_dir` | `String` | `"./static"` |
| `spa_apps` | `Option<HashMap<String, String>>` | `None` |
| `timeout_secs` | `u64` | `30` |
| `layers` | `Option<Vec<(i32, String)>>` | `None` |
| `enable_api_doc` | `bool`（api-doc feature） | `true` |

> `enable_cors` / `timeout_secs` 只是开关与数值，真正启用对应中间件需在 `layers` 中列出 `cors` / `timeout`。

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `WebConfig` | `conf`, `init`, `init_sort = i32::MAX` | 服务器参数；`address()`/`socket_addr()`/`static_dir()` 辅助方法 |
| `WebPlugin` | `app_async_init`, `app_async_run`, `init_sort = i32::MAX` | 路由合并、中间件装配、`axum::serve` 启动 |

**关键导出**：`Router`（类型别名，api-doc 时为 `aide::axum::ApiRouter`）、`DiComp<T>`（handler 提取器）、`WebErr`、`WebErrCode`、`add_layer`/`BodySizeLimitLayer`。

## 使用方式

```rust
use tx_di_core::{app, BuildContext};
use tx_di_axum;   // 必须：触发注册
use tx_di_log;    // 推荐

app! { AppModule }

// 必须在 BuildContext::new() 之前注册路由
fn register_routes() {
    use tx_di_axum::{WebPlugin, Router};
    use axum::routing::get;
    let r: Router = Router::new().route("/hello", get(|| async { "hi" }));
    WebPlugin::add_router(r);
}

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    register_routes();
    let mut ctx = BuildContext::new::<std::path::PathBuf>(Some("configs/app.toml"));
    let app = ctx.build_and_run().await?;   // 启动 Web 服务器

    // handler 内注入组件
    use tx_di_axum::{DiComp, WebConfig};
    async fn status(cfg: DiComp<WebConfig>) -> String {
        cfg.address()
    }
    Ok(())
}
```

## 注意事项

1. **必须 `use tx_di_axum;`**：否则组件不注册，服务器不启动。
2. **路由注册时机**：`WebPlugin::add_router()` 必须在 `BuildContext::new()` 之前调用，之后注册无效。
3. **中间件排序**：`layers` 中 `(i32, name)` 值越小越靠近 Handler（内层），越大越靠外（先接收请求）。
4. **双栈绑定**：`create_tcp_listener` 用 `socket2` 绑定，`SO_REUSEADDR=true`，IPv6 时双栈。
5. **`api-doc` 默认开启**：会暴露 `/docs`，生产建议关闭 feature 或 `enable_api_doc=false`。
6. `WebPlugin.router` 为 `OnceLock`，仅能 set 一次；测试可用 `clear_routers()`/`clear_layers()`。
7. 必须在 `#[tokio::main]` 中运行，主线程需保持（如 `ctrl_c().await`）否则进程退出。
8. 业务层统一用 `tx_di_core::ApiR` / `tx_common::ApiRes` 作为响应体（本插件未导出 `R<T>` 类型，旧 README 中的 `R<T>`/`/di` 端点示例不适用）。
