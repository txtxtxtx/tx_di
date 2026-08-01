# rsipstack 0.5.22 详细分析报告

> 分析对象：cargo 缓存 `~/.cargo/registry/src/rsproxy.cn-e3de039b2554c837/rsipstack-0.5.22/`
> 关联确认：tx_di workspace `Cargo.toml` 声明 `rsipstack = { version = "0.5" }`，`Cargo.lock` 锁定 **0.5.22**
> ⚠️ 之前记忆记录为 0.5.7，实际编译使用 0.5.22 —— 本报告以 0.5.22 为准，并标注相对 0.5.7 的新增能力
> 分析日期：2026-08-01

---

## 1. 总体结论

rsipstack 是一个**功能完整、工程化程度高**的 SIP 协议栈（RFC 3261 核心 + RFC 3581 rport + RFC 6026 + RFC 5923 alias + RFC 5626 outbound），四层架构清晰：

```text
┌─────────────────────────────────────────┐
│  Dialog Layer    （UAC/UAS 全状态机）    │  ← 最丰富的一层
├─────────────────────────────────────────┤
│  Transaction Layer（8 种 Timer/状态机）  │  ← 最完整的一层
├─────────────────────────────────────────┤
│  Transport Layer （UDP/TCP/TLS/WS）      │  ← 可插拔钩子多
├─────────────────────────────────────────┤
│  SIP Message     （Parser/Headers/URI）  │  ← 零拷贝解析
└─────────────────────────────────────────┘
```

**关键判断：rsipstack 已经把「UAS INVITE 会话管理」「UAC INVITE 全流程」「NAT 学习」「dialog 持久化恢复」全部实现了。** tx_di_sip 需要做的是「编排 + 暴露 + 生命周期管理」，而不是重新实现 —— 这大幅降低了优化成本（无需重写，只需接线）。

---

## 2. 源码结构总览（0.5.22，~7500 行，含测试）

| 目录 | 文件 | 职责 |
|------|------|------|
| `src/sip/` | message / parser / uri / method / status_code / headers / transport / version | SIP 消息模型 + 解析器（`Headers` 是 typed/untyped 混合 Vec） |
| `src/transaction/` | endpoint / transaction / key / timer / message | **Endpooint 协调器 + Transaction 状态机** |
| `src/dialog/` | dialog / dialog_layer / client_dialog / server_dialog / registration / invitation / subscription / publication / authenticate | **对话层（最丰富）** |
| `src/transport/` | transport_layer / connection / udp / tcp / tcp_listener / tls / websocket / stream / channel / sip_addr | 传输层 |
| `src/resolver/` | sip_resolver | SIP DNS（NAPTR/SRV/A） |
| `src/error.rs` | Error 枚举（9 变体） | thiserror 派生 |

---

## 3. 事务层（Transaction）深度剖析

### 3.1 Transaction 结构

```rust
pub struct Transaction {
    pub transaction_type: TransactionType,  // Client/Server × Invite/NonInvite
    pub key: TransactionKey,                // branch + method + ... 
    pub original: Request,                  // 初始请求（可 Clone）
    pub state: TransactionState,            // Calling/Trying/Proceeding/Completed/Confirmed/Terminated
    pub connection: Option<SipConnection>,  // 已选连接
    pub last_response: Option<Response>,
    pub last_ack: Option<Request>,
    // Timer A/B/C/D/G/K 句柄
    pub tu_sender: TransactionEventSender,  // TU 事件通道
    pub tu_receiver: TransactionEventReceiver,
}
```

### 3.2 状态机（RFC 3261 完整实现）

- **ClientInvite**：Nothing → Calling → Trying/Proceeding → Completed(收到 2xx 自动发 ACK) → Terminated
- **ServerInvite**：Trying → Proceeding → Completed(回 2xx 后等 ACK，Timer G 重传) → Confirmed(收到 ACK) → Terminated
- **Timer 语义**：A=UDP 重传(指数退避, 上限 t1x64=32s)、B=非可靠超时(→408)、C=INVITE 超时(→408)、D=ServerInvite 完成等待、G=2xx 重传、K=ACK 等待

### 3.3 对 tx_di 最重要的 5 个事务层事实

| # | 事实 | 对 tx_di_sip 的意义 |
|---|------|---------------------|
| 1 | **超时自动回 408**：Timer B/C 到期时 `inform_tu_response(408)`，`receive()` 会收到 408 响应 | 出站 MESSAGE **不会无限挂起**（最长 t1x64=32s）。之前报告「无限等待」判断有误，需修正为「最长 32s 且可通过 EndpointOption.t1 缩短」 |
| 2 | **UDP 自动重传 + 重查连接**：Timer A 触发时若 `connection.is_none()` 会重新 lookup 并重发 | 事务级断线重试天然存在 |
| 3 | **ACK 吸收**：`finished_transactions` 缓存最后消息，2xx 后重复 INVITE 重发响应、ACK 静默吸收 | 上层无需处理重发/重复 ACK |
| 4 | **send() 失败不返回 Err**：连接不可用也进入 Calling，交给 Timer A 重试 | 出站 API 的 Err 只表示协议错误，不表示发送失败 |
| 5 | **EndpointOption 可配置超时**：`t1/t4/t1x64/timerc/callid_suffix` | tx_di_sip 应暴露这些配置（当前完全没用） |

---

## 4. 对话层（Dialog）深度剖析 —— 本项目金矿

### 4.1 三种 Dialog 角色 API 全景

| 能力 | ClientInviteDialog (UAC) | ServerInviteDialog (UAS) | 说明 |
|------|--------------------------|--------------------------|------|
| 建立 | `DialogLayer::do_invite` / `do_invite_async` | `DialogLayer::get_or_create_server_invite` | 都有 |
| 应答 | — | `accept(h,b)` / `accept_with_public_contact` / `reject(code,reason)` / `ringing()` | 同步，NAT 感知 |
| 终止 | `bye()` / `hangup()` / `cancel()` | `bye()` / `bye_with_reason()` | cancel 需 Calling/Early |
| in-dialog 请求 | `reinvite/update/info/options/request/notify/refer/message` | 同左 + `handle(tx)` 自动分发 ACK/BYE/INFO/OPTIONS/UPDATE/PRACK/RE-INVITE | **双向都有** |
| 状态 | `state()` → `DialogState`（12 变体） | 同左 | TransactionHandle 用于 INFO 等异步回复 |
| 持久化 | `snapshot()` + `DialogLayer::restore_from_snapshot` | 同左 | **只恢复 Confirmed** |
| 查询 | `DialogLayer::get_client_dialog_by_call_id` | `DialogLayer::match_dialog(tx)` | in-dialog 路由 |

### 4.2 DialogState 完整状态集（12 变体）

```rust
pub enum DialogState {
    Calling(DialogId), Trying(DialogId),
    Early(DialogId, Response),
    WaitAck(DialogId, Response),          // UAS 已回 2xx 等 ACK
    Confirmed(DialogId, Response),        // 会话确立
    Updated/Notify/Info/Options/Refer/Message(DialogId, Request, TransactionHandle),
    Terminated(DialogId, TerminatedReason),
}
```

### 4.3 ⚠️ 两个必须注意的契约（影响 GB28181 现有代码）

1. **do_invite 成功后必须 remove_dialog**：2xx 后 dialog 注册进 DialogLayer 内部 `dialogs: DashMap`，**调用方必须监听 Terminated 后调 `DialogLayer::remove_dialog`，否则内存泄漏**。tx_di_gb28181 的 3 处 do_invite（invite_internal/snapshot/对讲）都**没有调 remove_dialog** —— 长期运行点播/回放会缓慢泄漏。
2. **DialogGuardForUnconfirmed 自动 cancel**：`do_invite` 内部在未确认阶段持有 guard，若调用方提前 drop 会**自动发 CANCEL 并等待最终响应**。这也是内存安全的兜底。

### 4.4 UAS 侧 `ServerInviteDialog::handle()` 已实现的完整逻辑

`handle(&mut self, tx)` 自动处理：CSeq 校验（丢弃旧请求）、Confirmed 后的 BYE/INFO/OPTIONS/UPDATE/PRACK/RE-INVITE/Message/Notify/Refer 分发、WaitAck 期间的 BYE、非确认态其他请求忽略。**UAS 需要的状态机 100% 在 rsipstack 里，tx_di_sip 只需要串起来。**

---

## 5. 注册层（Registration）深度剖析

```rust
pub struct Registration {
    pub credential: Option<Credential>,
    pub public_address: Option<HostWithPort>,  // ← NAT 学习结果
    pub call_id: CallId,                        // ← 固定 Call-ID（重注册一致）
    pub outbound_proxy: Option<SocketAddr>,
    pub contact: Option<Contact>,
}
impl Registration {
    pub async fn register(&mut self, server: Uri, expires: Option<u32>) -> Result<Response>;
    pub fn discovered_public_address(&self) -> Option<HostWithPort>;  // NAT 穿透
    pub fn expires(&self) -> u32;                                     // 从 Contact 解析
    pub fn create_nat_aware_contact(username, public, local) -> Contact; // 静态工具
}
```

**重要**：`register()` 自动处理 401/407（handle_client_authenticate 重发）、**从 401/200 的 Via received/rport 学习公网地址并重写 Contact**、支持 outbound_proxy 固定出口（NAT 场景关键）。

⚠️ **tx_di_sip 缺陷确认**：`SipSender::register()` 每次 `Registration::new()` —— **NAT 学习结果每次丢失、Call-ID 每次变化**（真实设备重注册 Call-ID 应保持一致）。SipClient 应持有单个 Registration 实例跨周期复用。

## 6. 传输层（Transport）深度剖析

### 6.1 结构与可插拔钩子

```rust
pub trait DomainResolver { async fn resolve(&self, target: &SipAddr) -> Result<SipAddr>; }   // 默认: SIP DNS (NAPTR/SRV/A)
pub trait TransportWhitelist { async fn allow(&self, ip: IpAddr) -> bool; }                   // ← 传输层 IP ACL
pub trait MessageInspector {                                                                  // ← 出/入站消息钩子
    fn before_send(&self, msg: SipMessage, dest: Option<&SipAddr>) -> SipMessage;
    fn after_received(&self, msg: SipMessage, from: Option<&SipAddr>) -> SipMessage;
}
pub trait TargetLocator { async fn locate(&self, uri: &Uri) -> Result<SipAddr>; }             // ← 自定义路由
pub trait TransportEventInspector { async fn handle(&self, e: TransportEvent) -> Option<TransportEvent>; }
```

| 能力 | 说明 | tx_di 价值 |
|------|------|-----------|
| `TransportWhitelist` | 入站 IP 白名单（UDP 包/TCP 连接级） | GB 平台可做**传输层设备 IP 白名单**（比业务层中间件更早拦截） |
| `MessageInspector` | before_send/after_received 全量消息钩子 | **完美接入点**：日志、metrics、NAT 头修正、消息计数 |
| `TargetLocator` | 覆盖默认解析逻辑 | 预留：将来 GB 级联路由/多网卡出口 |
| `TransportEvent::New/Closed` | TCP 连接建立/断开事件 | **断线感知**：Closed 事件驱动重连逻辑 |
| `connections: DashMap<SipAddr, SipConnection>` | TCP 连接复用表 | 出站 TCP 复用（`lookup`） |

### 6.2 传输类型

`SipConnection` 枚举：`Channel / Udp / Tcp / TcpListener / Tls / TlsListener / WebSocket / WebSocketListener`，统一 `send(msg, dest)` + `is_reliable()`。

---

## 7. Endpoint / EndpointBuilder 能力清单

```rust
EndpointBuilder::new()
    .with_user_agent("...")
    .with_transport_layer(tl)
    .with_cancel_token(token)
    .with_timer_interval(Duration)          // 定时器精度
    .with_allows(vec![Method])              // Allow 头
    .with_option(EndpointOption)            // ← t1/t4/t1x64/timerc 可配置！
    .with_inspector(Box<dyn MessageInspector>)
    .with_target_locator(Box<dyn TargetLocator>)
    .with_transport_inspector(Box<dyn TransportEventInspector>)
    .with_domain_resolver(Box<dyn DomainResolver>)
    .build() -> Endpoint;

Endpoint { inner: EndpointInnerRef }        // 可 Clone、可跨任务共享
EndpointInner::get_stats() -> EndpointStats // running/finished/waiting_ack
EndpointInner::get_running_transactions()
EndpointInner::get_via(addr, branch)        // ← 自动 RFC 5923 alias、rport
EndpointInner::make_request / make_response / make_ack
```

---

## 8. 0.5.7 → 0.5.22 新增能力（影响 tx_di_sip 设计）

| 能力 | 新增版本线索 | 说明 |
|------|-------------|------|
| `EndpointOption`（t1/t4/t1x64/timerc 配置化） | 0.5.22 | 事务超时从硬编码变可配 → tx_di_sip 应透传 |
| `MessageInspector` / `TargetLocator` / `TransportEventInspector` | 0.5.22 | 三个可插拔钩子 → metrics/路由/断线感知接入点 |
| `TransportWhitelist` | 0.5.22 | 传输层 IP 白名单 |
| `EndpointStats` + `get_stats()` | 0.5.22 | 现成运行指标 |
| `DialogSnapshot` + `restore_from_snapshot` | 0.5.22 | dialog 持久化恢复（重启恢复点播会话） |
| `do_invite_async` | 0.5.22 | 后台 INVITE（不阻塞调用方） |
| `accept_with_public_contact` | 0.5.22 | UAS NAT 感知应答 |
| `Registration.public_address` 自动学习 + `outbound_proxy` | 0.5.22 | NAT 客户端注册关键能力 |
| `get_via` + RFC 5923 alias | 0.5.22 | TCP NAT 连接复用 |
| `transaction_event_sender_noop()` | 0.5.22 | 恢复 dialog 时无 TU 通道兜底 |
| RFC 5626 Path/outbound（REGISTER 带 Supported: path,outbound） | 0.5.22 | 注册穿透 |

---

## 9. 错误模型

```rust
pub enum Error {
    SipMessageError(SipError), DnsResolutionError(String),
    TransportLayerError(String, SipAddr), TransactionError(String, TransactionKey),
    EndpointError(String), DialogError(String, DialogId, StatusCode),
    IoError(io::Error), AddrParseError(AddrParseError), WebSocketError(...),
    Error(String),
}
// + 自动 From<SendError>/From<TrySendError> —— 无 channel 错误 boilerplate
```

⚠️ `DialogError` 携带 `StatusCode` —— tx_di_sip 可映射到 SipErr 或直接透传。

---

## 10. 关键结论（供优化方案引用）

1. **UAS INVITE 会话管理 = rsipstack 已内置**，tx_di_sip 只需封装 `get_or_create_server_invite + handle() + state_rx + remove_dialog` 四条命令。
2. **UAC INVITE = `do_invite_async` 更优**：GB 点播/回放可后台执行，且 `InviteOption.destination` 支持指定 SipAddr（多网卡场景）。
3. **NAT 支持 = Registration.public_address 自动学习**，前提是 **SipClient 持有单例 Registration**（当前 SipSender 每次新建导致学习丢失）。
4. **出站超时 = 32s 硬编码（t1x64）可配置**：`EndpointOption.t1` 改小即可缩短；`send_out_of_dialog` 的 `receive()` 超时会收到 408，不是无限挂起。
5. **内存泄漏风险**：do_invite 后未 remove_dialog —— 必须纳入 tx_di_sip 的 InviteHandle 生命周期管理。
6. **断线感知**：`TransportEventInspector` 或直接监听 `TransportEvent::Closed`；事务层 Timer A 也会自动重查连接 —— tx_di_sip 的「重连」主要是**注册层**的职责（重 REGISTER + 心跳），传输层无需重写。
7. **持久化**：`DialogSnapshot` 使「平台重启恢复进行中的点播会话」成为可能（GB 场景价值高）。

---

*报告完*
