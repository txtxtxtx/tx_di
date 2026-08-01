# tx_di_sip 插件分析报告与改进方案

> 分析对象：`plugins/tx_di_sip`（基于 rsipstack 0.5.7 的 SIP 传输层插件）
> 关联分析：`plugins/tx_di_gb28181`（服务端/上级平台）、`plugins/tx_di_gb_dev`（设备端）
> 分析日期：2026-08-01
> 结论：**能支撑，但「完整度 70%」** —— 传输层扎实、分层正确，但存在 2 个 P0 缺口（无 UAS INVITE 会话管理、SipSender::invite 丢弃 dialog 状态通道）、5 个 P1 问题、8 个 P2 增强点。详见下文。

---

## 1. 结论摘要

| 维度 | 评价 | 说明 |
|------|------|------|
| 能否支撑 GB28181 | ✅ 可以 | REGISTER/MESSAGE/INVITE/心跳/目录/级联/点播/回放均已跑通 |
| 完整性 | ⚠️ 70% | 缺 UAS INVITE 会话封装、缺 keepalive/重连、缺注册状态持久化、认证中间件缺客户端侧 |
| 架构合理性 | ✅ 80% | L0 纯净分层正确、SipTx 信封设计优秀、洋葱链中间件合理；有 3 处职责越界/冗余 |
| 需补充功能 | ✅ 是 | 见 §5 改进方案（P0 3 项 / P1 6 项 / P2 7 项） |

**一句话结论**：`tx_di_sip` 作为「L0 纯净 SIP 栈」定位准确、工程质量高，但它目前只覆盖了「传输 + 路由 + 出站原语」三层，缺了 SIP 服务器/UA 真正需要的「会话层 + 状态层 + 韧性层」——这三层恰好是 GB28181 这类复杂业务最需要的，目前被迫由上层（tx_di_gb28181 / tx_di_gb_dev）重复实现，导致代码重复、能力不一致。

---

## 2. tx_di_sip 现状盘点

### 2.1 模块清单（11 个文件，~1100 行）

| 模块 | 职责 | 质量评价 |
|------|------|---------|
| `comp.rs` | SipPlugin 主组件：传输层构建、Endpoint、分发循环 | ✅ 良好，背压设计到位 |
| `config.rs` | SipConfig/TlsConfig | ✅ 良好，IPv6/TLS/mTLS 都覆盖 |
| `handler.rs` | SipRouter：方法路由 + 优先级 + catch-all | ✅ 良好 |
| `sip_tx.rs` | SipTx 共享事务信封（幂等 reply/fake 测试桩） | ✅ 优秀，本项目最佳设计 |
| `middleware.rs` | 洋葱链中间件 | ✅ 良好 |
| `sender.rs` | SipSender 出站 API | ⚠️ 有缺陷（见 §4.2） |
| `client.rs` | SipClient 周期注册组件 | ❌ 孤儿组件，无人使用 |
| `dialog.rs` | DialogKey / InDialogTable | ✅ 良好（未被使用） |
| `auth.rs` | MD5 摘要认证原语 | ✅ 良好（从 GB 层下沉正确） |
| `err.rs` | SipErr 错误码 | ⚠️ 不完整（12 个码，缺超时/重连/INVITE 会话等） |
| `metrics.rs` | SipMetrics 快照 | ❌ 未接线（无任何生产代码引用） |

### 2.2 已覆盖能力矩阵

| 能力 | 状态 | 说明 |
|------|------|------|
| UDP/TCP/TLS/WS 传输 | ✅ | 含 mTLS、external_ip NAT 头改写 |
| 入站方法路由 | ✅ | REGISTER/MESSAGE/INVITE/… + catch-all + 405 兜底 |
| 中间件洋葱链 | ✅ | DI 收集 `as_trait = dyn SipMiddleware`，sort 排序 |
| 并发背压 | ✅ | bounded channel + Semaphore（可配置） |
| 优雅关闭 | ✅ | CancellationToken |
| 出站 REGISTER/INVITE/BYE/CANCEL | ✅ | 依赖 rsipstack Registration/DialogLayer |
| 出站 MESSAGE/NOTIFY/SUBSCRIBE/INFO | ✅ | 手工构造 Transaction（out-of-dialog） |
| 摘要认证服务端校验 | ✅ | auth.rs 原语 + GB 层中间件 |
| 摘要认证客户端（主动带 Authorization） | ✅ | build_digest_authorization（GB 层在用） |
| **UAS INVITE 会话管理** | ❌ | **无 API**（GB 层广播/对讲因此残缺，见 §3.3） |
| **SIP keepalive / OPTIONS 探活** | ❌ | **无**（依赖设备侧心跳，链路死连不可感知） |
| **断线重连 / 注册状态持久化** | ❌ | **无**（网络抖动后 SipClient 只能等下一个周期） |
| **SipClient 组件** | ⚠️ | 存在但**无任何上层使用**（孤儿组件） |

---

## 3. 与 GB28181 需求的对照分析

### 3.1 GB28181 服务端（tx_di_gb28181）对 sip 插件的实际依赖

```
tx_di_gb28181 (Gb28181Server, init_sort=10001)
  ├─ Arc<SipPlugin> ──→ sender() / add_handler() / get_cancel_token()
  │     ├─ sender().register/unregister      → 级联下级向上级注册
  │     ├─ sender().send_message             → 心跳/目录/设备控制/PTZ 等全部出网 MESSAGE
  │     ├─ sender().dialog_layer().do_invite → 点播/回放/抓拍/对讲（绕过 SipSender::invite！）
  │     └─ add_handler(REGISTER/MESSAGE/NOTIFY/SUBSCRIBE/OPTIONS)
  ├─ Gb28181AuthMiddleware (as_trait=dyn SipMiddleware, sort=10)  → 摘要认证前置
  └─ 心跳超时 watchdog（自实现）
```

### 3.2 已良好支撑的部分

1. **REGISTER 全流程**：认证中间件（401 质询 → 校验 → 200）→ 注册表 → 心跳刷新，链路完整。
2. **MESSAGE 全链路**：`SipSender::send_message` 支撑了心跳、目录、设备控制、级联目录推送等所有出网命令；入站 `handle_message` 分发 20+ 种 GB 指令。
3. **UAC INVITE**：点播/回放/抓拍/对讲全部跑通（虽然绕过 API，见 §4.2）。
4. **级联（CascadeLower）**：完全复用 `sender().register/unregister/send_message`。
5. **设备端（tx_di_gb_dev）**：注册/心跳/查询响应/PTZ 控制响应全部基于 SipSender + SipRouter，UAS INVITE/BYE 也通过 add_handler 实现（它是自己写的！证明 sip 插件缺 UAS 封装，见 §3.3）。

### 3.3 缺口映射（GB 需求 → sip 插件缺口）

| # | GB28181 需求 | 现状 | sip 插件缺口 |
|---|-------------|------|-------------|
| 1 | 语音广播/对讲：设备向平台 INVITE 推音频 | ❌ **无法建立**：平台未注册 INVITE handler，设备 INVITE 落到 catch-all → 405 | **P0：无 UAS INVITE 会话管理 API**（设备端 tx_di_gb_dev 自己在 invite.rs 里用 rsipstack 裸 API 实现，平台端没有） |
| 2 | 断网自动恢复注册 | ⚠️ 级联/设备端靠周期重试，间隔长（expires/2） | **P1：无重连机制**（SIP 层无 keepalive、无 backoff 重试） |
| 3 | 平台主动探测设备在线 | ❌ 依赖心跳超时（120s+），被动 | **P1：无 OPTIONS 探活 API** |
| 4 | 点播/回放/抓拍 dialog 状态通知 | ⚠️ 绕过 SipSender 手动 do_invite + 手搓 state_rx 循环（3 处重复代码） | **P0：SipSender::invite 丢弃 state_receiver** |
| 5 | 注册状态持久化（平台重启设备自动重注册） | ❌ 设备端注册状态在内存 | **P1：无注册状态模型** |
| 6 | 出网 MESSAGE 等待响应 | ⚠️ send_message 返回 Response 但无超时控制（可能无限等） | **P1：send_out_of_dialog 无超时/无重试** |
| 7 | 大消息分片（GB 目录 >MTU） | ⚠️ 未处理 | P2：见改进方案 |

## 4. 架构合理性评估

### 4.1 做得好的设计（值得保留）

1. **L0 纯净分层**：`tx_di_sip` 不感知任何 GB 语义，`auth.rs` 从 GB 层下沉正确，`SipClient` 也是零 GB 语义 —— 这条边界必须守住。
2. **SipTx 信封设计**：解决 `Transaction: !Clone + &mut self` 的传递难题，`fake()` 测试桩让 handler/中间件可纯内存单测。这是全项目最好的抽象之一。
3. **中间件 DI 收集**：`as_trait = dyn SipMiddleware` + `inject_all_traits_from_store`，每个 App 实例独立中间件集合，避免全局 REGISTRY 串台。
4. **背压双保险**：bounded channel（队列满 await）+ Semaphore（并发上限），可配置化。
5. **InDialogTable / DialogKey**：正确的半键→完整键合并模型，为 UAS 会话管理打好了基础（可惜没接线）。
6. **幂等回复**：`AtomicBool` 保证首个回复真正发送，配合 405 兜底，杜绝 Transaction Drop 无响应。

### 4.2 问题清单（按严重度分级）

#### P0 — 必须修（影响 GB28181 核心功能）

| # | 问题 | 位置 | 后果 |
|---|------|------|------|
| P0-1 | **SipSender::invite 丢弃 state_receiver**：`let (state_sender, _state_receiver) = ...` | sender.rs:141 | 上层无法拿到 DialogState（Confirmed/Terminated）通知 → 被迫绕过 API 用 dialog_layer 手搓（tx_di_gb28181 有 **3 处重复代码**：invite_internal / snapshot / 对讲） |
| P0-2 | **无 UAS INVITE 会话管理**：没有 server-dialog 封装，没有「收到 INVITE → 回 200 → 关联 InDialogTable → 收 BYE 清理」的完整链路 | 整个插件 | GB28181 语音广播/对讲（设备→平台 INVITE 推音频）**无法建立**；设备端 tx_di_gb_dev 被迫用 rsipstack 裸 API 自己写（invite.rs 69 行），平台端直接没写 |
| P0-3 | **无 INVITE/BYE 入站处理器注册**（GB28181 层）：`register_server_handlers` 只注册 5 个方法，设备 INVITE → 405 | handlers.rs（GB 层） | 广播音频流必然失败 |

#### P1 — 应该修（影响生产可用性）

| # | 问题 | 位置 | 后果 |
|---|------|------|------|
| P1-1 | **SipClient 是孤儿组件**：全 workspace 搜索无任何上层使用 | client.rs | 重复实现蔓延：tx_di_gb_dev 自己写注册+心跳循环，CascadeLower 自己写注册+目录循环，三套逻辑不一致 |
| P1-2 | **send_out_of_dialog 无超时**：`while let Some(msg) = tx.receive().await` 无限等待 | sender.rs:328 | 网络黑洞时调用方永久挂起（GB 层 PTZ/控制命令会卡死 handler） |
| P1-3 | **retry_count / request_timeout_secs 是占位配置**：README 自认「实际重试由 rsipstack 内部处理」 | config.rs | 配置欺骗：用户设置了超时/重试，实际不生效 |
| P1-4 | **无 keepalive/断线重连**：SIP 传输层无 OPTIONS 探活，TCP 断连后 SipClient/级联要等 expires/2 才重试 | 整个插件 | 设备/平台掉线感知慢（分钟级） |
| P1-5 | **auth.rs 是 MD5 手写实现**：RFC 2617 虽只要求 MD5，但手写 MD5 有侧信道/实现错误风险 | auth.rs | 建议换 `md-5` crate（几行代码，消除审计风险） |
| P1-6 | **注册状态不持久化**：REGISTER 状态只存在 GB 层 DeviceRegistry 内存 | GB 层 + 无 sip 层状态模型 | 平台重启后设备必须等注册过期才重注册 |

#### P2 — 可以优化（增强/体验）

| # | 问题 | 位置 |
|---|------|------|
| P2-1 | metrics.rs 未接线（SipMetrics 无人读、无 /metrics 输出） | metrics.rs |
| P2-2 | `sender()` 每次调用重建 SipSender（OnceLock 只缓存了 DialogLayer） | comp.rs |
| P2-3 | 出站 Via 头构造硬编码 transport 字符串，未走 rsipstack 的 via 工具 | sender.rs:287 |
| P2-4 | 入站 dispatch 每次 clone 中间件 Vec（`RwLock` 读锁 + clone） | handler.rs:226 |
| P2-5 | 无大消息分片（GB 目录 > MTU 需 TCP，但无分片/确认重传机制） | — |
| P2-6 | SipErr 缺超时/重连/UAS 会话相关错误码（只有 12 个） | err.rs |
| P2-7 | 无测试覆盖出站路径（sender 无测试，client 无测试） | — |

## 5. 完善改进方案

> 优先级：**P0（本周可做，共 3 项）→ P1（两周内，共 6 项）→ P2（持续增强，共 7 项）**。
> 每项给出：目标、改动文件、关键 API 设计、代码骨架。

### 5.1 P0-1：SipSender::invite 返回 dialog 状态通知

**目标**：让上层拿到 `DialogStateReceiver`，消灭 3 处手搓 `do_invite` 重复代码。

**改动**：`sender.rs` 新增导出类型 `InviteHandle`，`invite()` 改为返回它：

```rust
// sender.rs
/// INVITE 会话句柄：对话框 + 状态通知通道
#[derive(Clone)]
pub struct InviteHandle {
    pub dialog: ClientInviteDialog,
    /// 监听 Confirmed/Terminated 状态（配合 tokio::select! 消费）
    pub state_rx: DialogStateReceiver,
}

impl SipSender {
    pub async fn invite(
        &self,
        caller: &str,
        callee: &str,
        sdp_offer: Option<Vec<u8>>,
        credential: Option<Credential>,
    ) -> RIE<InviteHandle> {
        let dialog_layer = self.dialog_layer();
        let (state_sender, state_rx) = dialog_layer.new_dialog_state_channel();
        let (dialog, _resp) = dialog_layer
            .do_invite(InviteOption { /* 同现状 */ }, state_sender)
            .await
            .map_err(|_| SipErr::InviteFailed)?;
        Ok(InviteHandle { dialog, state_rx })
    }
}
```

**配套（tx_di_gb28181）**：`invite_internal` / `snapshot` / 对讲三处改用它，删除重复的 `do_invite + state_rx 循环`（约 -80 行重复代码）。

### 5.2 P0-2：新增 UAS INVITE 会话管理（服务端对话层）

**目标**：补齐「平台/设备作为 UAS 接收 INVITE」的完整能力 —— 这是广播/对讲建立的前提。

**改动**：新增 `server_dialog.rs` 模块（复用现有 `dialog.rs` 的 `InDialogTable`）：

```rust
// server_dialog.rs —— 服务端 INVITE 会话管理
/// 服务端邀请会话：由 get_or_create_server_invite 创建，200 OK 后关联业务上下文
pub struct ServerInviteSession {
    key: DialogKey,                    // Call-ID + From-tag (+To-tag)
    tx: Transaction,                   // 用于 reply 200 / BYE
    pub created_at: Instant,
}

/// UAS INVITE 管理器（DI 组件注入，绑定到 SipPlugin 生命周期）
#[derive(Component)]
#[component(init_sort = 10000)]
pub struct SipServerDialog {
    #[tx_cst(skip)]
    sessions: InDialogTable<ServerInviteSession>,
    #[tx_cst(skip)]
    pending: DashMap<String, ServerInviteSession>,  // 半键 → 等待 ACK
}

impl SipServerDialog {
    /// 供 INVITE handler 调用：接受或拒绝
    pub async fn accept(&self, tx: SipTx, sdp_answer: &[u8], state_tx: DialogStateSender) -> RIE<()>;
    pub async fn reject(&self, tx: SipTx, code: StatusCode) -> RIE<()>;
    /// BYE handler 调用：清理会话
    pub async fn on_bye(&self, tx: SipTx) -> RIE<()>;
    /// 业务层查询活跃 UAS 会话
    pub fn active_sessions(&self) -> Vec<ServerInviteSession>;
}
```

**配套（tx_di_gb28181）**：`register_server_handlers` 增加 `INVITE` 与 `BYE` 注册，INVITE handler 内：

```rust
sip_plugin.add_handler(Some("INVITE"), 0, move |tx: SipTx| {
    // 解析 SDP s= 字段 → Play / Playback / Broadcast / Talk
    // 分配 RTP 端口 → build_sdp_answer → server_dialog.accept(tx, sdp, state_tx)
    // state_rx 监听 Terminated → 关闭 RTP 端口 + 事件
});
```

> 设备端 `tx_di_gb_dev/invite.rs` 的裸实现可以反向迁移到该模块，两角色共用一套 UAS 逻辑。

### 5.3 P0-3：激活 SipClient（消灭孤儿组件 + 统一注册/心跳/重连）

**目标**：让 `tx_di_gb_dev`、`CascadeLower` 复用 `SipClient`，一套代码三处受益（注册、心跳、重连、注销）。

**改动**：`client.rs` 增加两个可配置回调 + 重连退避：

```rust
// SipClientConfig 增加
#[serde(default)]
pub keepalive_secs: u32,          // 心跳间隔（0 = 只注册不心跳）
#[serde(default = "default_max_retries")]
pub max_retries: u32,             // 连续失败退避上限，默认 5
#[serde(default = "default_backoff_base")]
pub backoff_base_secs: u64,       // 指数退避基数，默认 2（2^N 秒）

// SipClient 增加
pub fn on_keepalive<F>(&self, f: F) where F: Fn(SipSender) -> RIE<()> + Send + Sync + 'static;
// 注册成功/失败回调，供 GB 层同步注册状态
pub fn on_registered<F>(&self, f: F) where F: Fn(bool) + Send + Sync + 'static;
```

**生命周期改造**（app_async_run 内）：

```text
注册 ──失败──▶ 指数退避重试（2s/4s/8s/…上限 max_retries）
  │
  └──成功──▶ 周期任务：每 keepalive_secs 调 on_keepalive（GB 层发 Keepalive MESSAGE）
             每 expires/2 调 register() 续期
             连续失败 N 次 → 进入退避重连
  │
  └──cancel──▶ unregister() + 收尾
```

**配套**：`tx_di_gb_dev` 改为注入 `Arc<SipClient>` + 注册 `on_keepalive` 回调（发心跳），删掉自己 343 行的 `register.rs` 循环逻辑；`CascadeLower` 同样改为回调模式。

### 5.4 P1-1：出站请求超时 + 重试（让配置真正生效）

**目标**：`send_out_of_dialog` 与 `invite` 增加超时，`retry_count`/`request_timeout_secs` 从占位变成真实现。

**改动**：`sender.rs`：

```rust
/// 带超时的出站请求（复用现有 send_out_of_dialog）
async fn send_out_of_dialog_timeout(&self, ...) -> RIE<rsip::Response> {
    let timeout = Duration::from_secs(self.config.request_timeout_secs.max(1));
    let mut last_err = None;
    for attempt in 0..=self.config.retry_count {
        match tokio::time::timeout(timeout, self.send_out_of_dialog_raw(...)).await {
            Ok(Ok(resp)) => return Ok(resp),
            Ok(Err(e)) => last_err = Some(e),      // 立即失败：不重试（协议错误）
            Err(_elapsed) => {                       // 超时：指数退避后重试
                last_err = Some(SipErr::RequestTimeout.into());
                tokio::time::sleep(Duration::from_millis(100 << attempt)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| SipErr::MessageFailed.into()))
}
```

**注意**：MESSAGE 等幂等命令可安全重试；INVITE 重试需谨慎（rsipstack DialogLayer 自己处理 401/重试，上层调用 `invite()` 不重复包重试）。

### 5.5 P1-2：OPTIONS 探活 + 连接健康检测

**目标**：SIP 层主动探测对端在线，TCP 断连快速感知。

**改动**：`sender.rs` 增加探活 API，`SipClient` 集成：

```rust
/// 向对端发送 OPTIONS 探活，返回是否收到 200
pub async fn ping(&self, to: &str) -> RIE<bool> {
    let resp = self.send_out_of_dialog(rsip::Method::Options, to, self.config.contact_uri(), None, None, vec![]).await?;
    Ok(resp.status_code == StatusCode::OK)
}
```

配合 `SipClient` 心跳循环：心跳失败 → `ping()` 连续 2 次失败 → 判定断连 → 进入重连退避。

### 5.6 P1-3：注册状态模型（SipRegistrationState）

**目标**：让「注册状态」成为一等公民，支撑平台重启恢复、多账号注册、状态查询 API。

**改动**：新增 `registration.rs`：

```rust
/// 注册状态（L0 纯净，无 GB 语义）
#[derive(Clone, Debug, Serialize)]
pub struct SipRegistration {
    pub registrar: String,
    pub username: String,
    pub registered: bool,
    pub expires: u32,
    pub last_success: Option<jiff::Timestamp>,
    pub last_error: Option<String>,
}

pub struct SipRegistrationStore {
    regs: DashMap<String /* username */, SipRegistration>,
}
impl SipRegistrationStore {
    pub fn upsert(&self, r: SipRegistration);
    pub fn get(&self, username: &str) -> Option<SipRegistration>;
    pub fn all(&self) -> Vec<SipRegistration>;
    pub fn mark_failed(&self, username: &str, err: &str);
}
```

`SipClient` 每次注册/失败时写入，`SipPlugin` 作为 DI 组件暴露，上层（GB 层）可查询所有注册状态，admin 后台直接展示。

### 5.7 P1-4：auth.rs 换 md-5 crate

```toml
# Cargo.toml
md-5 = "0.10"
```

```rust
// auth.rs —— 替换手写 MD5（保留公共函数签名，向后兼容）
use md5::{Md5, Digest};

pub fn md5_digest(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize().into()
}
```

### 5.8 P1-5：GB28181 层注册状态持久化配合

`tx_di_gb28181` 的 `DeviceRegistry` 增加 `dump()` / `load()`（已有 restore_devices 雏形），平台启动时从 DB 恢复设备注册状态，配合 §5.6 的 `SipRegistrationStore` 展示注册链路是否健康。

---

### 5.9 P2 增强清单（简要）

| # | 增强 | 说明 |
|---|------|------|
| P2-1 | metrics 接线 | `SipPlugin` 定时刷新 `SipMetrics` 到共享状态，或暴露 `metrics()` 方法，admin 后台 /healthz 读取 |
| P2-2 | SipSender 缓存 | `SipPlugin` 持有 `OnceLock<SipSender>`，`sender()` 直接 clone（Sender 本身 Arc 包装，clone 廉价） |
| P2-3 | Via 头构造 | 用 `rsipstack::sip::Via::new` + transport 枚举，避免硬编码字符串拼装 |
| P2-4 | 中间件缓存 | `set_middlewares` 时预构建排序后 Vec + 一次性 chain 闭包缓存，dispatch 不再每次 clone |
| P2-5 | 大消息分片 | 出站 MESSAGE body > 1300B 时提示/自动切 TCP 发送；入站支持 `application/message+sip` 分片重组 |
| P2-6 | 错误码扩充 | 增加 `RequestTimeout`(-13)、`NotRegistered`(-14)、`UasInviteFailed`(-15)、`TransportDown`(-16) |
| P2-7 | 测试补全 | sender 出站单测（fake endpoint）、SipClient 生命周期单测（fake SipPlugin）、server_dialog 单测 |

---

## 6. 分阶段实施路线图

```text
┌─ 阶段 0（P0，约 3-5 天）────────────────────────────────────┐
│ 1. SipSender::invite → InviteHandle（含 state_rx）         │
│ 2. 新增 server_dialog.rs（UAS INVITE 会话管理）            │
│ 3. tx_di_gb28181 注册 INVITE/BYE handler + 广播/对讲打通    │
│ 4. tx_di_gb28181 三处 do_invite 迁移到 InviteHandle         │
└────────────────────────────────────────────────────────────┘
   ┌─ 阶段 1（P1，约 1 周）──────────────────────────────────┐
   │ 5. SipClient 激活：keepalive/重连退避/回调              │
   │ 6. 出站超时 + retry_count 生效                          │
   │ 7. OPTIONS ping 探活集成                                │
   │ 8. SipRegistrationStore + DeviceRegistry dump/load      │
   │ 9. auth.rs 换 md-5 crate                               │
   └─────────────────────────────────────────────────────────┘
   ┌─ 阶段 2（P2，持续）─────────────────────────────────────┐
   │ metrics 接线 / sender 缓存 / 中间件缓存 / 错误码扩充     │
   │ / 大消息分片 / 测试补全                                 │
   └─────────────────────────────────────────────────────────┘
```

**验收标准**：
- 阶段 0 完成 → 广播/对讲端到端可用（设备 INVITE → 平台 200 → 音频流 → BYE 清理）。
- 阶段 1 完成 → 断网 30s 内自动重连恢复注册；出站命令不再无限挂起；admin 可查注册状态。
- 阶段 2 完成 → 5000 设备规模下 metrics 可观测、无中间件 clone 热点。

---

## 7. 附：快速评估表（30 秒版）

| 问题 | 答案 |
|------|------|
| 能支撑 GB28181 吗？ | ✅ 主体能，广播/对讲缺 UAS INVITE |
| 完整吗？ | ⚠️ 70%，缺会话层/状态层/韧性层 |
| 架构合理吗？ | ✅ 分层正确，工程质量高；SipClient 孤儿、invite API 残缺是主要瑕疵 |
| 要补充功能吗？ | ✅ P0 3 项 / P1 6 项 / P2 7 项，见 §5 |
| 最优先做什么？ | SipSender::invite 返回 InviteHandle + UAS server_dialog 模块 |

---

*报告完*
