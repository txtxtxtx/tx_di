# tx_di_sip — SIP 协议栈插件使用文档

基于 [rsipstack](https://crates.io/crates/rsipstack) 的 SIP 协议栈集成插件，是 `tx-di` 最底层的 L0 纯净 SIP 栈（不感知任何 GB28181 语义）。

## 用途

- 构建 UDP/TCP（可选 TLS/WS）传输层，启动 rsipstack `Endpoint`。
- 类 axum 路由的入站消息分发：`SipRouter::add_handler(method, priority, closure)`。
- 作用在 `SipTx` 上的中间件洋葱链（认证/日志/NAT/限流）。
- 统一出站发送接口 `SipSender`（register/invite/send_message/notify/subscribe/bye/...）。
- 基于 `CancellationToken` 的优雅停止。

上层插件（`tx_di_gb28181`、`tx_di_gb_dev`）在其之上构建具体协议。

## 启用

`Cargo.toml`：

```toml
tx_di_sip = { path = "plugins/tx_di_sip" }
# 可选 feature: rustls (TLS 传输), websocket (WS 传输)
```

## 配置

TOML 节名为 `[sip_config]`（可选 `[sip_client]` 用于主动向上级注册）：

```toml
[sip_config]
host = "0.0.0.0"
port = 5060
transport = "both"       # udp / tcp / both / tls / ws
user_agent = "MyApp/1.0"
external_ip = "203.0.113.10"   # NAT 公网 IP（可选）
log_messages = false
enabled = true
dispatch_queue_size = 10000
max_concurrent_handlers = 1000

[sip_config.tls]         # 仅 transport="tls" 时需要
cert_pem = "server.pem"
key_pem  = "server.key"

[sip_client]             # 可选：主动向上级注册
registrar = "sip:192.168.1.1:5060"
username  = "34020000001320000001"
password  = "12345678"
realm     = "3402000000"
expires   = 3600
enabled   = true
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `host` | `String` | `"0.0.0.0"` |
| `port` | `u16` | `5060` |
| `transport` | `SipTransport`（`udp`/`tcp`/`both`/`tls`/`ws`） | `Both` |
| `user_agent` | `String` | `"tx-di-sip/1.0.0"` |
| `external_ip` | `Option<String>` | `None` |
| `log_messages` | `bool` | `false` |
| `realm` | `Option<String>` | `None` |
| `retry_count` | `u32` | `1` |
| `request_timeout_secs` | `u64` | `30` |
| `tls` | `Option<TlsConfig>` | `None` |
| `enabled` | `bool` | `true` |
| `dispatch_queue_size` | `usize` | `10000` |
| `max_concurrent_handlers` | `usize` | `1000` |

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `SipConfig` | `conf`, `init_sort = 10000` | 服务端配置 |
| `SipPlugin` | `app_async_init`, `shutdown`, `init_sort = 10000` | 核心：传输层/Endpoint/入站分发/`SipSender` |
| `SipRouter` | `init_sort = 10000` | 入站路由（按方法名分发、catch-all、中间件链） |
| `SipClient` | `app_async_run`, `shutdown`, `init_sort = 20000` | 客户端 UA（周期注册续期） |
| `SipClientConfig` | `conf = "sip_client"`, `init_sort = 20000` | 客户端配置 |

关键非 Component 类型：`SipTx`（入站事务信封，`reply`/`request`/`take_transaction`）、`SipSender`（出站）、`SipMiddleware` trait（`as_trait = dyn SipMiddleware` 注册）。

## 使用方式

```rust
use tx_di_sip::{SipPlugin, SipRouter, SipTx};
use rsipstack::sip::StatusCode;
use tx_di_core::{BuildContext, app};

app! { AppModule }

// 1) 启动前注册入站处理器
fn register() {
    SipRouter::new().add_handler(Some("REGISTER"), 0, |tx: SipTx| async move {
        println!("收到 REGISTER");
        tx.reply(StatusCode::OK).await?;
        Ok(())
    });
}

#[tokio::main]
async fn main() -> tx_di_core::RIE<()> {
    register();
    let mut ctx = BuildContext::new::<std::path::PathBuf>(Some("configs/di-config.toml"));
    let app = ctx.build_and_run().await?;

    // 3) 启动后取发送器（endpoint 已就绪）
    let sip = app.inject::<SipPlugin>();
    let sender = sip.sender()?;
    sender.send_message("sip:alice@example.com", "sip:bob@example.com", b"hi", "text/plain").await?;
    Ok(())
}
```

上层插件常见做法：在自身 `app_async_init` 中注入 `Arc<SipPlugin>`，调用 `sip.add_handler(...)` 注册业务 handler，需要主动信令时 `sip.sender()`。

## 注意事项

1. **init 顺序强约束**：`SipClient`(20000) 必须晚于 `SipPlugin`(10000)，否则 `sender()` 取端点失败。
2. **`sender()` 时序**：只能在 `build_and_run()`/`ins_run()` 之后（Endpoint 已建）调用，否则返回"未设置 sip 端点"错误。
3. **传输层**：默认 `both` 同时绑定 UDP+TCP；`host="::"` 多数系统自动双栈。`transport="tls"` 需 `rustls` feature + `[sip_config.tls]`；`ws` 需 `websocket` feature。
4. **NAT**：`external_ip` 用于 Contact/Via 头；未设则用 `host`。
5. **幂等回复**：`SipTx::reply` 用 `AtomicBool` 保证只发首个回复；链结束仍无人回复自动回 405。
6. `enabled=false`：`SipPlugin` 跳过监听（纯客户端模式）；`SipClient.enabled=false` 跳过注册。
7. `retry_count`/`request_timeout_secs` 主要作配置占位，实际重试由 rsipstack 内部处理。
8. `SipMiddleware` 通过 `#[component(as_trait = dyn SipMiddleware)]` 注册，`SipPlugin::app_async_init` 用 `inject_all_traits_from_store` 收集并注入 `SipRouter`。
