# tx_di_sip 优化 / 重写详细方案（v2）

> 前置文档：`RSIPSTACK_ANALYSIS.md`（rsipstack 0.5.22 能力分析）、`ANALYSIS_REPORT.md`（现状问题清单）
> 本方案回答三个问题：**① sip 插件为 GB28181 平台提供什么能力；② 如何用 rsipstack 现成能力最小成本补齐；③ 模块级重写设计（Rust 最佳实践）。**
> 日期：2026-08-01

---

## 第 1 部分：先明确 —— tx_di_sip 为 GB28181 平台提供什么能力

### 1.1 定位（一句话）

> **tx_di_sip 是「GB28181 平台的 SIP 引擎」：屏蔽 rsipstack 的事务/对话框细节，向业务层提供「收发 SIP 信令、管理媒体会话、维持注册存活」三大类开箱即用的能力，全程零 GB 语义污染。**

### 1.2 能力契约（GB28181 平台视角的功能清单）

| 能力域 | 能力 | 对应 GB28181 使用场景 | 现状 |
|--------|------|----------------------|------|
| **A. 信令收发** | ① 入站方法路由（REGISTER/MESSAGE/INVITE/BYE/OPTIONS/...） | 设备注册/心跳/命令/点播/挂断 | ✅ 有 |
| | ② 出站 MESSAGE（含等待响应+超时） | 平台→设备 全部控制命令（PTZ/目录/录像/预置位/广播） | ⚠️ 无超时配置 |
| | ③ 出站 REGISTER/UNREGISTER（自动 401 重认证） | 级联下级、设备端注册 | ✅ 有 |
| | ④ 摘要认证（服务端校验 + 客户端构造） | 平台认证设备；设备/下级向平台注册 | ✅ 有（手写 MD5） |
| **B. 媒体会话** | ⑤ UAC INVITE（点播/回放/抓拍/对讲），返回 dialog + 状态流 | 平台主动发起 | ⚠️ 丢弃状态流 |
| | ⑥ **UAS INVITE（接收设备推流：广播/对讲音频）** | 语音广播/对讲 | ❌ 缺失 |
| | ⑦ BYE/CANCEL 挂断与清理（含 dialog 表防泄漏） | 挂断点播/回放 | ⚠️ 未 remove_dialog |
| | ⑧ in-dialog 请求（INFO/UPDATE/reINVITE） | 会话内信令 | ⚠️ 未暴露 |
| **C. 注册存活** | ⑨ 周期注册续期 + 心跳回调 | 设备端/级联的注册+Keepalive | ⚠️ SipClient 孤儿 |
| | ⑩ 断线重连（指数退避） | 网络抖动自动恢复 | ❌ 缺失 |
| | ⑪ 注册状态模型（可查询/可持久化） | admin 后台展示注册链路 | ❌ 缺失 |
| | ⑫ OPTIONS 探活 | 主动探测设备在线 | ❌ 缺失 |
| **D. 平台能力** | ⑬ NAT 穿透（Contact/Via 公网地址学习） | 设备在 NAT 后注册 | ⚠️ 每次新建丢学习 |
| | ⑭ IP 白名单（传输层 ACL） | 只允许可信设备 IP 接入 | ❌ 缺失（用 `TransportWhitelist`） |
| | ⑮ 运行时指标（事务数/消息计数/会话数） | 监控/健康检查 | ❌ metrics 未接线 |
| | ⑯ dialog 持久化恢复 | 平台重启恢复点播 | ❌ 缺失（rsipstack 已支持） |

**优先级排序（按 GB28181 业务价值）**：⑥ UAS INVITE > ⑤ InviteHandle > ⑩ 重连 > ⑨ SipClient 激活 > ② 超时 > ⑪ 状态模型 > ⑭ IP 白名单 > ⑮ metrics > ⑯ 持久化 > ⑧ in-dialog > ⑫ 探活。

---

## 第 2 部分：总体设计（Rust 最佳实践）

### 2.1 设计原则

1. **不重写 rsipstack 已实现的东西** —— UAS/UAC 状态机、NAT 学习、定时器、重传全部复用，只做「编排 + 暴露 + 生命周期」。
2. **接口最小化、契约显式化** —— 每个公共 API 有明确的状态/生命周期契约（谁负责 remove_dialog、谁负责超时）。
3. **类型驱动** —— 用 `InviteHandle`、`RegStatus` 等强类型返回值替代裸元组，消除「丢弃 state_receiver」这类 API 缺陷。
4. **依赖注入 + 组件化** —— 延续 tx-di 风格：新能力均为 `#[derive(Component)]`，通过 DI 收集（中间件、探活器）。
5. **可测试** —— 每个公共 API 配 fake/桩（复用 SipTx::fake 模式）。

### 2.2 目标模块架构（v2）

```text
plugins/tx_di_sip/src/
├── lib.rs            # 公共导出 + 模块声明
├── config.rs         # SipConfig v2（新增 t1/t4/timerc/timeout/whitelist 配置）
├── comp.rs           # SipPlugin v2（Endpoint 构建 + 分发循环 + 钩子接线）
├── handler.rs        # SipRouter（保持，微调：中间件缓存）
├── sip_tx.rs         # SipTx（保持）
├── middleware.rs     # SipMiddleware（保持）
├── sender.rs         # SipSender v2（InviteHandle / 超时 / Registration 持有）
├── server_dialog.rs  # ★新增 UAS 会话管理（SipUasManager）
├── client.rs         # SipClient v2（激活：持有 Registration + keepalive 回调 + 退避重连）
├── registration.rs   # ★新增 SipRegistrationStore（注册状态模型）
├── transport.rs      # ★新增 传输层构建（Whitelist / Inspector / EndpointOption）
├── metrics.rs        # SipMetrics v2（接线：MessageInspector + EndpointStats）
├── dialog.rs         # DialogKey / InDialogTable（保持，UAS 表开始使用）
├── auth.rs           # 换 md-5 crate（签名不变）
└── err.rs            # SipErr v2（+Timeout/-13、+NotRegistered/-14、+UasInviteFailed/-15、+TransportDown/-16）
```

### 2.3 与 GB28181 插件的依赖关系（不变式）

```text
tx_di_gb28181 ──注入──▶ SipPlugin (init_sort=10000)
    │                     ├─ SipSender（出站）
    │                     ├─ SipUasManager（UAS 会话）★
    │                     └─ SipRouter（入站分发）
tx_di_gb_dev ──注入──▶ SipClient (init_sort=20000)  ★ 替代自己写的注册循环
CascadeLower ──注入──▶ SipClient / SipRegistrationStore ★
gb28181_admin ──注入──▶ SipMetrics + SipRegistrationStore ★
```

## 第 3 部分：模块级详细设计

### 3.1 config.rs v2 —— 让 rsipstack 配置可控

```rust
#[derive(Debug, Clone, Deserialize, Component)]
#[component(conf, init_sort = 10000)]
pub struct SipConfig {
    // ... 现有字段保持 ...
    /// 事务层定时器（透传 EndpointOption）
    #[serde(default = "default_t1_ms")]
    pub t1_ms: u64,                 // 默认 500，RTT 估计
    #[serde(default = "default_t1x64_ms")]
    pub t1x64_ms: u64,              // 默认 32000，最大超时（出站请求上限）
    #[serde(default = "default_timerc_secs")]
    pub timerc_secs: u64,           // 默认 180，INVITE 事务超时
    /// 出站 MESSAGE 应用层超时（秒），0 = 用事务层 t1x64
    #[serde(default = "default_out_timeout_secs")]
    pub outbound_timeout_secs: u64, // 默认 5 —— GB 控制命令不应等 32s
    /// 传输层 IP 白名单（可选）：命中才允许入站
    #[serde(default)]
    pub ip_whitelist: Vec<String>,  // ["192.168.1.0/24", "10.0.0.5"]
}

impl SipConfig {
    pub fn endpoint_option(&self) -> EndpointOption {
        EndpointOption {
            t1: Duration::from_millis(self.t1_ms),
            t4: Duration::from_secs(5),
            t1x64: Duration::from_millis(self.t1x64_ms),
            timerc: Duration::from_secs(self.timerc_secs),
            callid_suffix: None,
        }
    }
}
```

**要点**：把 rsipstack 的 `EndpointOption` 透传，让「出站最长等多久」真正可配；`outbound_timeout_secs` 用于应用层兜底（PTZ 命令 5s 无响应即返回错误，不阻塞 handler）。

### 3.2 transport.rs（新增）—— 构建时接线钩子

```rust
//! 传输层构建：Whitelist / MessageInspector / EndpointOption

/// IP 白名单（传输层 ACL，比业务中间件更早拦截）
#[derive(Clone)]
pub struct SipIpWhitelist { rules: Vec<cidr::IpCidr> }   // cidr = "0.3" crate 或手写掩码
#[async_trait]
impl TransportWhitelist for SipIpWhitelist {
    async fn allow(&self, ip: IpAddr) -> bool {
        self.rules.iter().any(|r| r.contains(&ip))
    }
}

/// 消息钩子：metrics 采集 + 可选 NAT 修正
pub(crate) struct SipInspector {
    metrics: Arc<SipMetricsInner>,   // 原子计数器
    external_ip: Option<IpAddr>,     // 重写入站 Via received
}
impl MessageInspector for SipInspector {
    fn before_send(&self, msg: SipMessage, _dest: Option<&SipAddr>) -> SipMessage {
        self.metrics.out_sent.fetch_add(1, Ordering::Relaxed);
        msg
    }
    fn after_received(&self, msg: SipMessage, _from: Option<&SipAddr>) -> SipMessage {
        self.metrics.in_recv.fetch_add(1, Ordering::Relaxed);
        msg
    }
}
```

**comp.rs 构建流程 v2**：

```rust
let transport_layer = TransportLayer::new_with_domain_resolver(token.clone(), resolver);
if !config.ip_whitelist.is_empty() {
    transport_layer.set_whitelist(Arc::new(SipIpWhitelist::try_new(&config.ip_whitelist)?)); // rsipstack 0.5.22 有 setter
}
let endpoint = EndpointBuilder::new()
    .with_cancel_token(token.clone())
    .with_transport_layer(transport_layer)
    .with_user_agent(&config.user_agent)
    .with_option(config.endpoint_option())          // ★ 超时可配
    .with_inspector(Box::new(SipInspector::new(metrics, config.external_ip))) // ★ metrics 接线
    .build();
```

### 3.3 sender.rs v2 —— InviteHandle + 超时 + Registration 持有

```rust
/// INVITE 会话句柄：dialog + 状态流 + 生命周期守卫
#[derive(Clone)]
pub struct InviteHandle {
    pub dialog: ClientInviteDialog,
    pub call_id: String,
    state_rx: Arc<Mutex<Option<DialogStateReceiver>>>,  // 可被 take 一次
    dialog_layer: Arc<DialogLayer>,
    removed: Arc<AtomicBool>,
}

impl InviteHandle {
    /// 取状态流（消费后本句柄不再提供）；业务方在 Terminated 后调用 cleanup
    pub fn take_state_rx(&self) -> Option<DialogStateReceiver>;

    /// 从 DialogLayer 移除 dialog（防泄漏）；Drop 时自动调用
    pub fn cleanup(&self) {
        if !self.removed.swap(true, Ordering::SeqCst) {
            self.dialog_layer.remove_dialog(&self.dialog.id());
        }
    }
}
impl Drop for InviteHandle {
    fn drop(&mut self) { self.cleanup(); }  // ★ RAII 防泄漏
}

impl SipSender {
    /// v2：返回 InviteHandle（含状态流 + 自动 cleanup）
    pub async fn invite(
        &self,
        caller: &str,
        callee: &str,
        sdp_offer: Option<Vec<u8>>,
        credential: Option<Credential>,
    ) -> RIE<InviteHandle> {
        let dialog_layer = self.dialog_layer();
        let (state_tx, state_rx) = dialog_layer.new_dialog_state_channel();
        let (dialog, _resp) = dialog_layer
            .do_invite(InviteOption { /* caller/callee/contact/offer/... */ }, state_tx)
            .await
            .map_err(|_| SipErr::InviteFailed)?;
        Ok(InviteHandle { dialog, call_id: dialog.id().call_id.clone(),
                          state_rx: Arc::new(Mutex::new(Some(state_rx))),
                          dialog_layer, removed: Arc::new(AtomicBool::new(false)) })
    }

    /// v2：后台 INVITE（GB 点播/回放不阻塞调用方）
    pub fn invite_async(&self, caller, callee, sdp, cred)
        -> RIE<(ClientInviteDialog, JoinHandle<InviteAsyncResult>)>
    { /* 包一层 do_invite_async */ }

    /// v2：出站 MESSAGE 带应用层超时（默认 outbound_timeout_secs）
    pub async fn send_message(&self, to, from, body, ct) -> RIE<Response> {
        let timeout = Duration::from_secs(self.config.outbound_timeout_secs.max(1));
        tokio::time::timeout(timeout, self.send_message_raw(to, from, body, ct))
            .await
            .map_err(|_| SipErr::RequestTimeout)?
    }

    /// v2：OPTIONS 探活
    pub async fn ping(&self, to: &str) -> RIE<bool> { /* 见 ANALYSIS_REPORT §5.5 */ }
}

/// ★ 注册持有（NAT 学习不丢失）：由 SipClient 持有，不再每次新建
pub struct RegistrationHandle {
    pub reg: Mutex<Registration>,     // 单例跨周期
    pub store: Arc<SipRegistrationStore>,
    pub username: String,
}
impl RegistrationHandle {
    pub async fn register(&self, server: &str, expires: Option<u32>) -> RIE<Response>;
    pub async fn unregister(&self) -> RIE<Response>;
    pub fn discovered_public(&self) -> Option<HostWithPort>;  // NAT 学习结果
}
```

### 3.4 server_dialog.rs（新增）—— UAS INVITE 会话管理

```rust
//! 服务端会话管理：接收设备 INVITE → 业务应答 → 状态通知 → 清理

/// UAS 邀请会话上下文（业务侧通过 SipUasManager 获取/控制）
#[derive(Clone)]
pub struct UasSession {
    pub dialog: ServerInviteDialog,     // rsipstack 原生句柄
    pub call_id: String,
    pub device_id: String,              // 从 From 提取
    pub sdp_offer: Vec<u8>,             // 请求 SDP（解析 s= 类型）
    pub created_at: Instant,
}

/// UAS INVITE 管理器（DI 组件，注入 SipPlugin 生命周期）
#[derive(Component)]
#[component(init_sort = 10000)]
pub struct SipUasManager {
    #[tx_cst(skip)]
    sessions: InDialogTable<UasSession>,            // 复用现有 dialog.rs
    #[tx_cst(skip)]
    dialog_layer: OnceLock<Arc<DialogLayer>>,
}

impl SipUasManager {
    /// INVITE handler 内调用：创建会话（回 100 Trying），返回应答决策权
    pub fn on_invite(
        &self,
        tx: &SipTx,                       // 入站事务（take_transaction 前）
        device_id: &str,
    ) -> RIE<UasSession> {
        let dialog_layer = self.dialog_layer();      // 需要 endpoint inner → 由 SipPlugin 注入
        let (state_tx, _rx) = dialog_layer.new_dialog_state_channel();
        // rsipstack 同步 API：生成 ServerInviteDialog
        let mut transaction = tx.take_transaction().ok_or(SipErr::TransactionMissing)?;
        let dlg = dialog_layer.get_or_create_server_invite(&transaction, state_tx, None, None)
            .map_err(|_| SipErr::UasInviteFailed)?;
        let session = UasSession {
            dialog: dlg.clone(),
            call_id: dlg.id().call_id.clone(),
            device_id: device_id.to_string(),
            sdp_offer: transaction.original.body.clone(),
            created_at: Instant::now(),
        };
        // 关联表：in-dialog BYE/INFO 查表
        let key = DialogKey::from_request(&transaction.original).unwrap().with_to_tag(&dlg.id().local_tag);
        self.sessions.insert(key, session.clone());
        Ok(session)
    }

    /// 接受（带 SDP answer；NAT 场景用 accept_with_public_contact）
    pub fn accept(&self, session: &UasSession, sdp_answer: &[u8], public: Option<HostWithPort>) -> RIE<()> {
        match public {
            Some(p) => session.dialog.accept_with_public_contact(
                &session.device_id, Some(p), &self.local_sip_addr(), None, Some(sdp_answer.to_vec())),
            None => session.dialog.accept(None, Some(sdp_answer.to_vec())),
        }.map_err(|_| SipErr::UasInviteFailed.into())
    }

    pub fn reject(&self, session: &UasSession, code: StatusCode) -> RIE<()>;

    /// BYE handler 内调用：结束会话并清理
    pub fn on_bye(&self, tx: &SipTx) -> RIE<()> {
        let key = DialogKey::from_request(tx.request()).ok_or(SipErr::InvalidUri)?;
        if let Some(s) = self.sessions.lookup(&key) {
            let _ = s.dialog.bye().await?; // 或直接 remove
            self.sessions.remove(&key);
        }
        tx.reply(StatusCode::OK).await
    }

    pub fn active_sessions(&self) -> Vec<UasSession>;
    pub fn by_call_id(&self, call_id: &str) -> Option<UasSession>;
}
```

**GB28181 接入（tx_di_gb28181 侧，配合改造）**：

```rust
// handlers.rs 增加
sip_plugin.add_handler(Some("INVITE"), 0, move |tx: SipTx| {
    let uas = uas_mgr.clone(); let server = server.clone();
    async move {
        // 1) 解析 SDP s= 字段 → Play/Playback/Broadcast/Talk
        // 2) 广播/对讲：分配 RTP 端口 → build_sdp_answer → uas.accept()
        // 3) 状态流监听 Terminated → 关 RTP 端口 + 事件 + cleanup
        Ok(())
    }
})?;
sip_plugin.add_handler(Some("BYE"), 0, move |tx: SipTx| {
    let uas = uas_mgr.clone();
    async move { uas.on_bye(&tx).await }
})?;
```

### 3.5 client.rs v2 —— SipClient 激活（持有 Registration + 回调 + 退避重连）

```rust
#[derive(Component)]
#[component(app_async_run, shutdown, init_sort = 20000)]
pub struct SipClient {
    pub config: Arc<SipClientConfig>,
    pub sip: Arc<SipPlugin>,
    /// 注册句柄（单例 Registration，NAT 学习/固定 Call-ID）
    #[tx_cst(OnceLock::new())]
    pub reg: OnceLock<Arc<RegistrationHandle>>,
    /// 心跳回调（GB 层注册：发 Keepalive MESSAGE）
    #[tx_cst(OnceLock::new())]
    pub keepalive_hook: OnceLock<Arc<dyn Fn(Arc<SipClient>) -> RIE<()> + Send + Sync>>,
    /// 注册状态（写 SipRegistrationStore）
    #[tx_cst(OnceLock::new())]
    pub store: OnceLock<Arc<SipRegistrationStore>>,
    #[tx_cst(OnceLock::new())]
    pub cancel_token: OnceLock<CancellationToken>,
}

// app_async_run 生命周期（替换现有循环）：
//
//  ┌─ register() ─失败─▶ 指数退避(2s,4s,8s...max_retries) 重试
//  │      │成功
//  │      ▼
//  │  写 store (registered=true) ──▶ 触发 on_registered 回调
//  │      │
//  │  循环 select!:
//  │    ├─ ticker(keepalive_secs)  → keepalive_hook()  // GB 心跳
//  │    ├─ ticker(expires/2)       → reg.register() 续期（复用 Registration → 401 自动重认证 + NAT 学习持续）
//  │    ├─ 连续失败 N 次          → 进入退避重连（重新 register）
//  │    └─ cancel_token           → unregister() + 收尾

/// 注册状态变更监听（级联/设备端同步自己的状态机）
impl SipClient {
    pub fn on_keepalive<F>(&self, f: F) -> RIE<()>;
    pub fn on_registered<F>(&self, f: F) -> RIE<()>;   // F: Fn(bool)
    pub fn registration(&self) -> Option<SipRegistration>;
}
```

**配套改造**：`tx_di_gb_dev` 注入 `Arc<SipClient>`，注册 `on_keepalive` 回调（内部用 `sip.sender().send_message` 发心跳），删除自写的 343 行注册/心跳循环；`CascadeLower` 同样改为注册回调。

### 3.6 registration.rs（新增）—— 注册状态模型

```rust
#[derive(Clone, Debug, Serialize)]
pub struct SipRegistration {
    pub username: String,
    pub registrar: String,
    pub registered: bool,
    pub expires: u32,
    pub public_addr: Option<String>,     // NAT 学习结果
    pub last_success: Option<jiff::Timestamp>,
    pub last_error: Option<String>,
    pub fail_count: u32,
}

#[derive(Component, Default)]
#[component(init_sort = 10000)]
pub struct SipRegistrationStore {
    #[tx_cst(skip)]
    regs: DashMap<String, SipRegistration>,
}
impl SipRegistrationStore {
    pub fn upsert(&self, r: SipRegistration);
    pub fn get(&self, username: &str) -> Option<SipRegistration>;
    pub fn all(&self) -> Vec<SipRegistration>;
    pub fn mark_success(&self, username: &str, expires: u32, public: Option<&str>);
    pub fn mark_failed(&self, username: &str, err: &str);
    // 持久化扩展：dump()/load() 供 admin 恢复
}
```

### 3.7 metrics.rs v2 —— 接线

```rust
pub struct SipMetricsInner {             // 原子计数（SipInspector 写入）
    pub in_recv: AtomicU64, pub out_sent: AtomicU64,
    pub reg_ok: AtomicU64, pub reg_fail: AtomicU64,
    pub invite_ok: AtomicU64, pub invite_fail: AtomicU64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SipMetrics {
    pub running: bool,
    pub handler_count: usize,
    pub registered_methods: Vec<String>,
    pub uptime_secs: u64,
    // v2 新增
    pub tx_running: usize, pub tx_finished: usize, pub tx_waiting_ack: usize,  // EndpointStats
    pub dialogs: usize,                // DialogLayer::len()
    pub sessions: usize,               // SipUasManager 活跃会话
    pub msg_in: u64, pub msg_out: u64,
    pub reg_ok: u64, pub reg_fail: u64,
    pub invite_ok: u64, pub invite_fail: u64,
}
impl SipPlugin { pub fn metrics(&self) -> RIE<SipMetrics>; }  // gb28181_admin 可注入
```

### 3.8 auth.rs —— 换 md-5 crate（签名不变）

```rust
// Cargo.toml 增加
md-5 = "0.10"

use md5::{Digest, Md5};
pub fn md5_digest(data: &[u8]) -> [u8; 16] {
    Md5::digest(data).into()
}
// md5_hex / verify_digest_auth / build_digest_authorization / extract_nonce / generate_nonce 全部不变
```

### 3.9 err.rs v2 —— 错误码扩充

```rust
#[err("SIP")]
pub enum SipErr {
    // ... 现有 -1~-12 不变 ...
    #[err(-13, "请求超时")]
    RequestTimeout,
    #[err(-14, "未注册")]
    NotRegistered,
    #[err(-15, "UAS INVITE 会话失败")]
    UasInviteFailed,
    #[err(-16, "传输层不可用")]
    TransportDown,
}
```

---

## 第 4 部分：实施里程碑与验收

### 4.1 里程碑

| 里程碑 | 内容 | 依赖 | 验收标准 |
|--------|------|------|---------|
| **M1 会话层**（3-5 天） | sender.rs InviteHandle + 超时；server_dialog.rs UasManager；GB 层注册 INVITE/BYE；3 处 do_invite 迁移 | 无 | 广播/对讲端到端可用；点播/回放无 dialog 泄漏（`DialogLayer::len()` 稳定） |
| **M2 存活层**（1 周） | SipClient 激活（Registration 单例 + keepalive 回调 + 退避）；RegistrationHandle；gb_dev/CascadeLower 迁移 | M1 | 断网 30s 自动重注册；NAT 后设备 Contact 持续正确；admin 可查注册状态 |
| **M3 平台层**（1 周） | transport.rs（Whitelist/Inspector/EndpointOption 透传）；SipRegistrationStore；metrics 接线；md-5；OPTIONS ping；错误码 | M1 | IP 白名单生效；metrics 输出正确；`retry_count/timeout` 配置真实生效 |
| **M4 增强**（持续） | DialogSnapshot 持久化恢复；in-dialog INFO/UPDATE 暴露；大消息分片；测试补全 | M2/M3 | 平台重启恢复 Confirmed 点播会话；5000 设备压力测试 |

### 4.2 Rust 最佳实践落地清单（对照）

| 实践 | 本方案落实点 |
|------|-------------|
| RAII 资源管理 | `InviteHandle::Drop → cleanup()`（remove_dialog），杜绝泄漏 |
| 类型化 API 消灭裸元组 | `(ClientInviteDialog, Option<Response>)` → `InviteHandle` |
| 显式超时 | 所有出站/等待路径包 `tokio::time::timeout` |
| 状态机回调 | `on_keepalive` / `on_registered` 闭包注入，替代继承/覆写 |
| 错误分层 | SipErr 扩到 16 码；rsipstack Error 只映射不吞 |
| 原子指标 | `AtomicU64` 计数 + 快照 struct（无锁） |
| 可测试性 | UasManager/RegistrationHandle 依赖 `EndpointInnerRef` 注入，可用 fake transport |
| 配置即契约 | SipConfig 全字段 `#[serde(default)]`，TOML 即文档 |

### 4.3 风险与注意

1. **rsipstack 0.5.x 版本演进**：`TransportLayer::set_whitelist` 等新 API 需以 0.5.22 为准核对签名；建议 `Cargo.toml` 锁 `=0.5.22` 或升级后回归。
2. **UAS `get_or_create_server_invite` 需要真实 Transaction**：`SipTx::take_transaction()` 后不可再 reply —— 顺序必须是「取事务 → 建 dialog → 业务决定 accept/reject」。
3. **`do_invite` 的 remove_dialog 契约**：InviteHandle 的 cleanup 与状态流消费要配套，避免「已 Terminated 又 remove」双重清理（`AtomicBool` 防重入）。
4. **迁移顺序**：先 M1 会话层（风险最高、价值最高），gb28181 三处手搓代码迁移时逐个替换、逐个验证，不要一把梭。

---

*方案完*
