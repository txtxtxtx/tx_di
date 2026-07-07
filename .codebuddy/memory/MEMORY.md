# tx_di 项目长期记忆

## 项目结构
- Rust 工作区，DI（依赖注入）框架
- `tx-di-macros`：proc-macro crate，提供 `#[derive(Component)]` 宏
- `tx-di-core`：核心运行时 crate（Component trait、Store、App、生命周期、拓扑排序、AOP）
- `common/`：通用工具 crate（tx_common、tx_error 等）
- `plugins/`：插件 crate（如 tx_di_log）
- `examples/`：示例应用（tx_admin 等）

## tx-di-macros 模块结构（2026-07-04 重构后）
原 `comp.rs`(703行单文件) + `utils.rs` 拆分为职责清晰的多模块：
- `attr/` — 属性解析（`comp_attr.rs` 解析 `#[component(...)]`，`field_attr.rs` 解析 `#[tx_cst(...)]`）
- `classify/fields.rs` — 字段分类 `FieldKind` 枚举
- `codegen/` — 代码生成（`mod.rs` 编排 + `CodeGenContext`，`component_impl.rs`、`factory.rs`、`inner_init.rs`、`meta_entry.rs`）
- `type_utils.rs` — 类型检测（Arc/Option/Arc<dyn Trait>）
- `name_utils.rs` — 命名转换（camel_to_snake 等）

数据流：属性解析 → 字段分类 → 构建 CodeGenContext → 各 codegen 子模块生成片段 → 组装

## 已知问题
- `examples/` 中部分 crate 引用 `tx_di_core::tx_comp`（不存在的宏），属预先存在的错误，与 Component derive 宏无关

## 测试
- `cargo test -p tx-di-core` 含 64 个测试覆盖宏全部功能路径
- 插件测试在各自 crate 中

## 框架 API 迁移（2026-07-05）

### 旧 API → 新 API
- `#[tx_comp(conf, init)]` → `#[derive(Component)] #[component(conf = "key", init)]`
- `#[tx_comp(init)]` → `#[derive(Component)] #[component(init)]`
- `impl CompInit for T { fn inner_init(...) }` → `fn init(this: &mut T, _store: &Store) -> RIE<()>`（模块级自由函数）
- `impl CompInit for T { async_method!(fn async_init_impl(ctx, token) { ... }) }` → `#[component(app_async_init)]` + `async fn app_async_init(comp: Arc<T>, app: Arc<App>) -> RIE<()>`
- `impl CompInit for T { async_method!(fn async_run_impl(ctx, token) { ... }) }` → `#[component(app_async_run)]` + `async fn app_async_run(comp: Arc<T>, app: Arc<App>, token: CancellationToken) -> RIE<()>`
- `impl CompInit for T { fn init_sort() -> i32 { N } }` → `#[component(init_sort = N)]`
- `InnerContext` → `Store`（init 回调参数）
- `ctx.inject::<T>()` → `comp.field`（直接访问字段）或 `app.inject::<T>()`

### 保留不变
- `#[tx_cst(expr)]` 字段自定义构造器仍受支持
- `RIE<T>` 类型别名不变
- `async_method!` 宏已移除，`BoxFuture` 包装由 `#[derive(Component)]` 生成代码自动处理

### 回调函数签名

| 属性 | 回调签名 |
|------|---------|
| `init` | `fn init(this: &mut T, _store: &Store) -> RIE<()>` |
| `app_init` | `fn app_init(comp: Arc<T>, app: &Arc<App>) -> RIE<()>` |
| `app_async_init` | `async fn app_async_init(comp: Arc<T>, app: Arc<App>) -> RIE<()>` |
| `app_async_run` | `async fn app_async_run(comp: Arc<T>, app: Arc<App>, token: CancellationToken) -> RIE<()>` |
| `shutdown` | `fn shutdown(_comp: &T)`（模块级自由函数，首参为组件引用，**不是** `&self`） |

### 注意事项
- `DepsTuple` 必须在所有使用 `#[derive(Component)]` 有依赖的模块中 `use tx_di_core::DepsTuple`
- `app` 参数在异步回调中必须是 `Arc<App>`（非 `&Arc<App>`），因为 `BoxFuture` 要求 `'static`
- 异步回调直接写 `async fn`，无需 `async_method!` 或手动 `Box::pin`

## rsipstack 头文件位置（0.5.x）
- 只有 `Via`/`From`/`To`/`CSeq` 在 `rsipstack::sip::typed`
- `CallId`/`ContentType`/`Event`/`Expires`/`MaxForwards` 等其余头在 `rsipstack::sip` 根（untyped 再导出），**不在** `typed`
- untyped 头均有 `.new(impl Into<String>)`；`Expires`/`MaxForwards` 另有 `From<u32>`
- `Endpoint { pub inner: EndpointInnerRef }`，`incoming_transactions(&self) -> Result<TransactionReceiver>`，`Transaction::new_client(key, request, endpoint_inner, None)` + `tx.send()` 发 out-of-dialog 请求

## tx_di_sip 插件（2026-07-06 实现）
- 架构：`SipTx`(`Arc<Mutex<Option<Transaction>>>` + 缓存 Request + `replied` 幂等标志 + `fake()` 测试模式) 解决 Transaction 不可克隆问题
- 中间件：`SipMiddleware` trait + `#[component(as_trait = dyn SipMiddleware)]` DI 收集，`build_chain` 洋葱模型
- `SipPlugin`(`#[component(app_async_init, shutdown)]`) + `SipRouter`(`#[component(init_sort=10000)]`) + `SipConfig`(`#[component(conf, init_sort=10000)]`)
- 性能参数 `enabled`/`dispatch_queue_size`/`max_concurrent_handlers` 在配置驱动（替代原环境变量 SipQueueSize/SIP_MAX_HANDLERS）

## tx_di_sip 插件架构决策（2026-07-06）

### 硬约束（rsipstack 0.5.16）
- `Transaction`：`Send + Sync + Unpin`，但 **`!Clone`**。
- 回复/发送方法均为 **`&mut self` + `async`**：`reply` / `reply_with` / `respond` / `send` / `send_cancel` / `send_ack`。
- `original: Request` 是公开字段（`Request` 可 `Clone`），只读检视零成本。
- `Drop for Transaction` 存在（服务端事务 Drop 会走清理/超时）→ **必须保证最终有 reply**。

### 用户明确要求：强绑定 rsipstack
- **不要**引入 `SipEndpoint` / `SipServer` 这类「解耦 rsipstack、可替换实现」的 trait。
- `SipPlugin` 直接持有 rsipstack `Endpoint`；Handler 接收真实 rsipstack 类型；gb28181 直接 `app.inject::<SipPlugin>()`。

### 中间件设计最终方案：SipTx 共享信封
- `Transaction` 不可克隆 + reply 需 `&mut`，故采用 `SipTx` 薄信封：`Arc<Mutex<Option<Transaction>>>` + 构造时缓存 `Request` 克隆 + `replied: Arc<AtomicBool>` 幂等标志 + 可选 fake 记录器。
- `SipTx` 生产用真实 `Transaction`，测试用 `SipTx::fake(Request)`（无需 `EndpointInnerRef`，reply 仅记录）→ 既强绑定又可单测。
- `SipMiddleware` trait：`async fn process(&self, tx: SipTx, next: SipNextFn) -> RIE<()>`，DI 收集（`inject_all_traits_from_store::<dyn SipMiddleware>()`），去掉原 `middleware.rs` 的全局 `static REGISTRY`。
- `SipHandlerFn = Arc<dyn Fn(SipTx) -> SipNextFut + Send + Sync>`；`router.dispatch` 用 `build_chain` 真正走洋葱链（`middleware.rs` 的 `apply_middleware_chain` 当前「接好没通电」——`handler.rs:305` 的 `router.dispatch(msg)` 没调用它，是已知 bug）。
- `dispatch` 兜底：链结束后若 `!replied` 则自动 reply 405，防止 Transaction Drop 无响应。
- `SipTx` 在 dispatch 末尾才 drop，确保真实 Transaction 的 Drop 发生在兜底回复之后。

### 其他已确认缺陷（待修）
- P4 `comp.rs` init 里 `ctx.inject::<SipPlugin>()` 自注入 → 改用 `app_async_init` 收 token。
- P5 shutdown 只 `token.cancel()` 未 join `JoinSet` → 用 `Arc<Mutex<JoinSet>>` 并在 `shutdown` await join。
- P6 配置缺 `enabled` 开关；P7 perf 参数（队列/并发）从环境变量改进 `SipConfig`。
- P8 `sender.rs` 返回 `anyhow::Result` → 统一 `RIE`；P9 补 `send_message/notify/subscribe/info`。
- P11 `SipMetrics` 是死代码，需实时快照接入。
- P12 `SipRouter` 应升为 DI Component（不再 `#[tx_cst(SipRouter::new())]` 藏在 SipPlugin 字段）。

## gb28181 插件架构决策（2026-07-06 重设计）
- 认证模块：`plugins/tx_di_gb28181/src/auth.rs` 的 `Gb28181AuthMiddleware` 实现 `SipMiddleware`（`#[component(as_trait = dyn SipMiddleware)]`），由 `SipPlugin::app_async_init` 经 `inject_all_traits_from_store::<dyn SipMiddleware>()` DI 收集进洋葱链（`sort()=10` 最外层前置）。
- `process` 仅对 REGISTER：ACL 前置 403 → 无 Authorization 发 401 质询（`NonceStore` 生成 nonce）→ 有则 `verify_digest_auth` 校验（失败 403，成功放行）；非 REGISTER 直接 `next(tx)`。
- `NonceStore` 现位于 `auth.rs`（随中间件单例常驻），`handlers.rs::handle_register` 不再含认证/ACL，只做注册/注销业务。
- 注意：`auth.rs` 需 `use rsipstack::sip::HeadersExt`（from_header/expires_header 等方法来自该 trait）。

### 真 BYE（2026-07-07 完成）
- `SessionInfo` 增加 `dialog: ClientInviteDialog` 字段（`rsipstack::dialog::client_dialog::ClientInviteDialog`，`#[derive(Clone)]` 且 thread-safe，可直接存 DashMap 跨任务）。因其未实现 `Debug`，`SessionInfo` 改为 `#[derive(Clone)]` + 手写 `Debug`（跳过 dialog 字段）。
- `hangup(call_id)` 真正发 SIP BYE：`self.sip_plugin.sender().bye(&sess.dialog)`；先 `sessions.remove` 再 close RTP + bye + emit `SessionEnded`。
- `invite_internal`/`audio_talkback` 的 `DialogState::Terminated` 分支加 `sessions_clone.contains_key` 去重，避免 hangup 后 Terminated 重复清理/事件。
- `plugin.rs` 的 `xml`/`sdp` 导入块是迁移时遗留未用项（调用都在 `plugin_tail.rs`），已删除；`media` 导入精简为 `{MediaBackend, build_backend}`。最终 `cargo check -p tx_di_gb28181` 零警告。

## tx_di_can 无设备联调增强（2026-07-08 完成）
用户无 CAN 硬件，需在 SimBus 上完整联调监控/诊断/刷写。核心交付：

### 后端（79 测试全绿）
- `src/db/` 描述库：内置 DTC/DID 集（VIN/SW/HW/EngineSpeed 等）+ 外部 JSON/TOML 加载，ECU 仿真与前端共用。
- `src/sim_ecu/` 通用 ECU 仿真节点：订阅接收→ISO-TP 重组→按描述库应答各 UDS SID（0x10/0x11/0x14/0x19/0x22/0x23/0x27/0x2E/0x31/0x34/0x36/0x37/0x3E）+ 最小 bootloader 状态机；仅 SimBus 或 `config.sim_ecu` 时启动。
- `src/sim_ecu/state.rs` 新增：显式擦除例程 0x31 0xFF00、压缩/加密真实协商（0x34 解析 dataFormatIdentifier，仅 0x00 支持）、修正地址/长度字段解析。
- `src/hex.rs` S19/IntelHEX 解码；`src/flash.rs` FlashError 真实触发 + 擦除/压缩加密接入 `FlashConfig`。
- `src/record.rs` 录制/回放 CSV；`src/dbc.rs` DBC 解析 + 信号字节↔物理量双向解码（Intel/Motorola）。
- `plugin.rs` 总线统计（帧计数/字节/负载率）+ 应用层帧过滤器；`event.rs` P2-2 评估结论：保留 CanEvent 为唯一真相源（tx-di-core 无应用事件总线）。

### 前端（vue-tsc 类型检查通过）
- 新增/增强 Tauri 命令：get_bus_stats/set_frame_filter/send_isotp/get_desc_dids/get_desc_dtcs/sim_ecu_status/record_csv/replay_csv/load_dbc/decode_dbc。
- 视图：TraceView（循环发送/过滤掩码/解码切换/负载率曲线/高亮/导出）、UdsView（+ISO-TP 原始面板 + 描述库）、FlashView（文件选择/擦除/seedkey）、SimEcuView（状态+自检）、RecordReplayView、DbcView。
- 验证命令：`cargo test -p tx_di_can`（79 绿）、`cargo check --workspace`、`npx vue-tsc --noEmit`（app 目录，需先 `npm install`）。

### tx_di_can 迁移与 A/B/C/D 完成（2026-07-08）
用户将 `tx_di_can` 从 `plugins/` 迁移至 `examples/tx_di_can`（属示例代码，非 workspace 插件），并完成四项高级特性：
- **A XCP 标定 + A2L**：`src/xcp.rs`（XcpSlave/XcpMaster/XcpPacket、A2L/Measurement/Characteristic 解析、CRO/DTO、SET_MTA/UPLOAD/DOWNLOAD/SHORT_UPLOAD/BUILD_CHECKSUM、DAQ/ODT）— 修复迁移后编译错（`map(|t| unquote(t))`、`slice4/slice2` 安全辅助替代 `try_into`、MTA 字段改名 `mta_addr`、解析仅在 `/begin` 触发）。
- **B 审计 + 报表**：`src/audit.rs`（`AuditEntry` + `OnceLock<Mutex<Vec>>`，record/ok/fail/log/clear）、`src/report.rs`（gen_html/export_html/export_pdf，手写 PDF 仅 ASCII 非 ASCII→`?`）。
- **C CSV 离线分析**：`src/record.rs` `analyze_csv` → `CsvAnalysis`（总帧数/FD帧/时间跨度/负载率‰/平均间隔/Top10 节点）。
- **D i18n + 工程管理**：`src/project.rs` `ProjectConfig`（`.canproj` JSON 保存/加载，含 CanConfig/Flash/最近 DID/DTC）；前端 `src/i18n.ts`（中英双语 localStorage 持久化）。
- 前端：新增 XcpView/AuditView/ProjectView，RecordReplayView 加离线分析面板，App.vue 加 xcp/audit/project 页签 + 语言切换按钮。
- 验证：`cargo test -p tx_di_can` **89 passed / 0 failed**；`cargo check -p can-host` EXIT=0；`npx vue-tsc --noEmit`（app 目录）EXIT=0。

### 已知待办（后续阶段，未实现）
产线权限分级、自动化脚本/宏、CCP 协议（仅 XCP on CAN 已做）。这些是计划 v3 项，本次未实现。
