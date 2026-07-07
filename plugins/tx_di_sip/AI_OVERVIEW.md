# tx_di_sip — AI 插件速览

> 面向 AI/维护者的快速理解文档。描述本插件的能力边界、公开 API、内部架构与集成要点。
> 权威信息以源码为准（`plugins/tx_di_sip/src/`）。

## 1. 定位

基于 [`rsipstack`](https://crates.io/crates/rsipstack) 的 **SIP 服务插件**，作为 `tx-di` DI 框架的组件提供开箱即用的 SIP 协议能力。它只负责**传输层 + 消息路由 + 发送接口 + 中间件**，不实现具体业务协议（GB28181 等由上层插件基于它构建）。

- 位置：`plugins/tx_di_sip/`
- 依赖核心：`rsipstack`（SIP 栈）、`tx-di-core`（DI）、`dashmap`、`tokio`、`tracing`
- 特性开关：`rustls`（TLS 传输）、`websocket`（WS 传输）

## 2. 公开 API（来自 `pub use`，见 `lib.rs`）

| 类型 | 角色 | 要点 |
|------|------|------|
| `SipPlugin` | 核心组件（DI 单例） | 启动 Endpoint、构建传输层、跑入站分发循环、暴露 `sender()` |
| `SipConfig` | 配置组件（`#[component(conf)]`） | TOML 驱动：`[sip_config]` |
| `SipRouter` | 消息路由器（DI 单例） | 按方法名注册 handler + catch-all，中间件洋葱链 |
| `SipTx` | 共享事务信封 | 包裹 `Transaction`，幂等 `reply`/`reply_with`，有 `fake()` 测试桩 |
| `SipSender` | 出站发送器 | `register/invite/bye/cancel/send_message/notify/subscribe/info` |
| `SipMiddleware` | 中间件 trait | `process(tx, next)` + `sort()` + `name()`，DI 收集 `as_trait = dyn SipMiddleware` |
| `SipMetrics` | 运行时指标快照 | `running/handler_count/registered_methods/uptime_secs` |
| `SipErr` | 错误码（`CodeMsg`，前缀 `"SIP"`） | `-1`~`-12` |

### 关键方法签名（速记）

```rust
// SipPlugin
SipPlugin::sender(&self) -> RIE<SipSender>              // 必须在 app_async_init 之后
SipPlugin::add_handler(&self, method: Option<impl AsRef<str>>, priority: i32, handler) -> RIE<()>

// SipSender（详细签名见 sender.rs）
register(registrar, username, password) -> RIE<Response>
invite(caller, callee, sdp_offer: Option<Vec<u8>>, credential: Option<Credential>)
    -> RIE<(ClientInviteDialog, Option<Response>)>
bye(&dialog: &ClientInviteDialog) -> RIE<()>           // 真 BYE（gb28181 挂断依赖此）
cancel(&dialog: &ClientInviteDialog) -> RIE<()>
send_message(to, from, body, content_type) -> RIE<Response>   // MESSAGE（国标级联核心）
notify(to, from, body, sub_state) -> RIE<Response>
subscribe(to, from, event, expires) -> RIE<Response>
info(to, from, body) -> RIE<Response>
inner() -> EndpointInnerRef                              // 高级用户直连 rsipstack
dialog_layer() -> Arc<DialogLayer>                       // 高级用户直连 DialogLayer

// SipTx（handler 内使用）
tx.method() -> Method
tx.request() -> &Request                                 // 只读副本，零锁
tx.reply(StatusCode) -> RIE<()>                          // 幂等
tx.reply_with(StatusCode, Vec<Header>, Option<Vec<u8>>) -> RIE<()>
tx.replied() -> bool

// SipMiddleware trait
async fn process(&self, tx: SipTx, next: SipNextFn) -> RIE<()>
fn sort(&self) -> i32 { 100 }                            // 越小越外层
fn name(&self) -> &str
```

## 3. 内部架构

### 3.1 生命周期（DI）
`SipPlugin` 用 `#[component(app_async_init, shutdown, init_sort = 10000)]`：
- `app_async_init`：从 `app.shutdown_token` 取 cancel token；构建传输层 + `Endpoint`；用 `inject_all_traits_from_store::<dyn SipMiddleware>()` 收集中间件注入 `SipRouter`；启动入站分发循环。
- `shutdown`：cancel token，分发循环据此优雅退出。

### 3.2 入站分发引擎（生产者/消费者）
- 生产者：`incoming_transactions()` → bounded `mpsc` channel（容量 `dispatch_queue_size`，默认 10000，队列满则背压）。
- 消费者：从 channel 取消息，用 `Semaphore`（许可数 `max_concurrent_handlers`，默认 1000）限制并发 handler；经 `SipRouter::dispatch` 分发。
- 兜底：链结束仍无人回复 → 自动 405，防止 `Transaction` Drop 无响应。

### 3.3 中间件洋葱链
- 中间件通过 DI 收集（`#[component(as_trait = dyn SipMiddleware)]`），**不再使用全局 REGISTRY**。
- `build_chain(mws, handler)` 按 `sort` 升序包裹，正序执行：外层 A → 外层 B → handler → 外层 B → 外层 A。
- `pre` 阶段可 `tx.reply()` 短路；`next(tx).await` 之后可做 post 日志/指标。

### 3.4 SipTx 设计动机
`rsipstack::Transaction` 是 `!Clone`、回复方法为 `&mut self + async`，无法在中间件链中按值传递或重试。故 `SipTx` 包 `Arc<Mutex<Option<Transaction>>>`，构造时缓存 `Request` 副本供只读；`reply` 用 `AtomicBool` 保证幂等（首个回复真正发送，后续忽略）。`fake()` 提供无 `Endpoint` 测试桩。

## 4. 配置（`[sip_config]`）

| 字段 | 默认 | 说明 |
|------|------|------|
| `host` | `"0.0.0.0"` | 监听地址，支持 IPv4/IPv6（`::` 双栈） |
| `port` | `5060` | SIP 端口 |
| `transport` | `udp` | `udp`/`tcp`/`both`/`tls`/`ws`（`tls`/`ws` 需对应 feature） |
| `user_agent` | `"tx-di-sip/1.0.0"` | UA 字符串 |
| `external_ip` | `None` | NAT 公网 IP，填 Contact/Via |
| `log_messages` | `false` | 详细消息日志 |
| `realm` | `None` | 认证域（部分服务器用设备 ID 作 realm） |
| `retry_count` | `1` | 请求超时重试 |
| `request_timeout_secs` | `30` | 请求超时（秒） |
| `tls` | `None` | `TlsConfig { cert_pem, key_pem }`（`transport=tls` 时必填） |
| `enabled` | `true` | 关闭则只做客户端（不监听入站） |
| `dispatch_queue_size` | `10000` | 分发队列容量 |
| `max_concurrent_handlers` | `1000` | 并发 handler 上限（Semaphore 背压） |

## 5. 集成要点（给其他插件/上层使用）

1. 在 `Cargo.toml` 依赖 `tx_di_sip`。
2. 上层插件通常 `#[component(...)]` 注入 `Arc<SipPlugin>`，在 `app_async_init` 中 `sip_plugin.add_handler(...)` 注册业务处理器。
3. 主动发信令：通过 `sip_plugin.sender()` 获取 `SipSender`（注意：须在 `build_and_run()` 之后，即 endpoint 已建好）。
4. 跨任务持有会话：用 `SipSender::invite` 返回的 `ClientInviteDialog`（它 `#[derive(Clone)]` 且 thread-safe），需挂断时 `sender.bye(&dialog)`。
5. 自定义中间件：实现 `SipMiddleware` + `#[component(as_trait = dyn SipMiddleware)]`，自动被收集进洋葱链；`sort` 越小越前置（认证类应极小，如 gb28181 用 `10`）。

## 6. 测试
`lib.rs` 内嵌单测：覆盖 `SipConfig` 解析/默认值、`SipTransport` 反序列化、`SipRouter` 注册/分发/优先级/清除、`SipTx` fake 幂等。
