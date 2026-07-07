# CAN 诊断上位机 需求分析（tx_di_can）

> 目标读者：嵌入式诊断工程师 / 项目负责人
> 目的：梳理"一个 CAN 诊断上位机需要哪些功能"，对照现有 `tx_di_can` 插件定位缺口，给出已确认范围的架构与分阶段交付计划。
> 状态：**v2 — 范围已确认**（见第 1 节决策）

---

## 1. 已确认的范围决策

| 项 | 决策 | 影响 |
|----|------|------|
| 界面形态 | **Tauri**：Rust 后端 + Web(Vue3/TS) 桌面 App | 复用 `tx_di_can` 引擎，跨平台，与项目 examples(Vue/TS) 技术栈一致 |
| 目标硬件/系统 | **PCAN（PEAK PCAN-Basic）+ Windows** 优先 | 适配器优先级 PCAN > SimBus(仿真) > SocketCAN(Linux 留作后续) |
| 协议范围 | **UDS-on-CAN + CAN-FD（含 FD 版 ISO-TP）** | 必须实现 64 字节 FD ISO-TP 与 escape 序列 |
| MVP 范围 | **监控 + 诊断 + 刷写 完整 MVP 一起做** | 三大模块同批交付，不先砍功能 |

> 待定（不阻塞 MVP）：Kvaser/Vector 适配器、DBC 解码、录制文件格式(BLF/ASC)、seed-key 插件化加载。这些列为 v2/v3 迭代。

---

## 2. 系统架构

### 2.1 分层

```
┌──────────────────────────────────────────────────────────┐
│  Tauri 桌面 App  (apps/can-diag)                          │
│  ┌────────────┐  ┌────────────┐  ┌─────────────────────┐  │
│  │ Vue3 + TS  │  │ Pinia 状态 │  │ 组件：Trace/UDS/...  │  │
│  │ 前端 UI    │  │ 管理        │  │                     │  │
│  └────────────┘  └────────────┘  └─────────────────────┘  │
│        │ invoke / listen (Tauri IPC)                      │
│  ┌──────────────────────────────────────────────────┐    │
│  │ src-tauri (Rust)                                  │    │
│  │  Tauri Commands ──► tx_di_can 门面 (CanPlugin)    │    │
│  │  Event 转发: CanEvent ──► app.emit(...) ──► 前端   │    │
│  │  BuildContext::build() 在 setup 中初始化 DI 容器   │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
                         │
            ┌────────────┴─────────────┐
            ▼                          ▼
   tx_di_can (协议引擎)        tx-di-core (DI 框架 / App / Store)
   adapter / isotp / uds /     事件体系 / 生命周期
   flash / frame
            │
   ┌────────┴─────────┬──────────┬──────────┐
   ▼                  ▼          ▼          ▼
 SimBus(仿真)    PCAN(Windows)  SocketCAN  (Kvaser 占位)
```

### 2.2 目录布局（建议，新增成员）

```
plugins/tx_di_can/
├── src/                     # 现有协议引擎库（改造）
├── app/                     # Tauri 应用根
│   ├── src-tauri/           # Rust 侧：commands + 事件转发 + tauri.conf.json
│   │   ├── src/main.rs      # setup: BuildContext::build(); 注册命令
│   │   ├── commands.rs      # #[tauri::command] 封装 CanPlugin 调用
│   │   ├── events.rs        # CanPlugin::on_event → app.emit
│   │   └── Cargo.toml       # 依赖 tauri, tx_di_can, tx-di-core
│   └── src/                 # Vue3 + TS 前端
│       ├── views/           # TraceView / UdsView / FlashView / ConfigView
│       ├── stores/          # Pinia
│       └── api/             # Tauri invoke 封装
└── REQUIREMENTS.md
```

### 2.3 前后端通信契约

- **指令（前端 → 后端）**：Tauri `#[command]`，同步/异步封装 `CanPlugin` 静态 API
  - `connect(config)` / `disconnect()`
  - `send_frame(frame)` / `send_fd_frame(frame)`
  - `read_did(tx_id, did)` / `write_did(tx_id, did, data)`
  - `read_dtc(tx_id)` / `clear_dtc(tx_id)`
  - `session_control(tx_id, mode)` / `ecu_reset(tx_id, kind)` / `tester_present(tx_id, on)`
  - `security_access(tx_id, level, key_algo)` / `run_routine(tx_id, rid, ...)`
  - `flash(firmware_path, config, key_algo)`（带进度事件）
- **事件（后端 → 前端）**：`CanPlugin::on_event` 收到的 `CanEvent` 经 `app.emit("can://event", ev)` 转发；前端 `listen("can://event", ...)` 接收
  - `FrameReceived` / `FdFrameReceived` → 实时刷新 Trace
  - `BusReady` / `BusError` → 连接状态
  - `UdsResponse` / `UdsProgress` / `FlashProgress` / `FlashDone` → 诊断/刷写进度
- **共享类型**：Rust 结构体用 `serde` 派生，`tauri::ipc` 自动序列化；前端用 TS 接口镜像（可用 `ts-rs` 自动生成，避免手写漂移）。

### 2.4 关键约束

- 后端是**全局单例**（`CanPlugin::instance()` / `OnceLock`），Tauri 命令直接调用即可，无需每命令重建。
- DI 容器只在 `setup` 调用一次 `BuildContext::build()`；命令内部通过 `tx_di_core::App` 获取组件。
- 前端**禁止**直接触达硬件；所有硬件操作经命令走后端。

---

## 3. 后端改造与缺口（必须先做）

按优先级排序，**在做 UI 之前或同步进行**：

| # | 缺口 | 现状（已核实） | 改造内容 |
|---|------|----------------|----------|
| P0-1 | **PCAN 接收回路** | `adapter.rs` PCAN 的 FFI 仅有 `CAN_Write`，**无 `CAN_Read`/`CAN_ReadFD`**，且无后台接收任务。`close()` 有 `tasks`/`running` 脚手架但未用。 | ① 补 `CAN_Read`/`CAN_ReadFD` FFI；② `open()` 后 `tokio::spawn` 接收任务，把帧 `tx.send()`；③ 处理 `running` 标志优雅退出。 |
| P0-2 | **迁移到新框架 API** | `plugin.rs` 用已废弃的 `#[tx_comp(init)]` / `CompInit` / `InnerContext` / `BoxFuture`。 | 改为 `#[derive(Component)] #[component(init, app_async_init, shutdown)]`，用 `Store`/`App` 替代；重接 `start_rx_loop`（其逻辑可保留）。目标：`cargo check` 通过。 |
| P0-3 | **FD 版 ISO-TP** | 经典 SF/FF/CF/FC 已有；无 64 字节 FD 传输与 escape 序列。 | `isotp.rs` 新增 FD 路径：64 字节数据帧、CI/FC 在 FD 帧、`escape` 序列处理；`IsoTpConfig` 加 `is_fd` 开关。 |
| P0-4 | **时间戳** | 适配器接收时帧时间戳恒为 0。 | 接收处填 `SystemTime`/`Instant`（PCAN/SocketCAN 优先用硬件/驱动时间戳，回退系统时钟）。 |
| P1-1 | **应用层过滤 + 统计** | 仅能按 `rx_id` 过滤，无 UI 层过滤/负载率。 | `adapter` 暴露帧计数；过滤逻辑放前端或后端 `subscribe` 包装。 |
| P1-2 | **DTC / DID 描述库** | `uds.rs` 只给原始码/字节。 | 新增 `db/` 模块：默认内置常用 DTC（0xPxxxx）、DID（0xF190 VIN、0xF195 SW 版本…），支持外部 JSON/TOML 加载。 |
| P1-3 | **seed-key 算法可配置** | 刷写 `key_fn` 是 Rust 闭包，UI 无法配。 | 内置 2~3 种常见算法（按 level 选择）；预留"加载 .dll/.so"接口（v3）。 |
| P1-4 | **刷写文件格式** | 仅 BIN。 | 加 S19 / Intel HEX 解析（复用 `flash.rs` 数据层，新增解码器）。 |
| P1-5 | **刷写擦除例程** | 流程缺显式 erase（0x31 0xFF00）。 | `FlashEngine` 增加 erase 步骤与压缩/加密协商字段。 |
| P2-1 | **录制 / 回放** | 无任何导出。 | 后端把 `CanEvent` 流落地为 CSV（MVP）；BLF/ASC 留 v3。回放命令重发 trace。 |
| P2-2 | **事件总线统一** | `event.rs` 用独立全局总线，与 `tx-di-core` 事件体系不一致。 | 评估并入 `tx-di-core` 事件，或至少对齐 gb28181 插件的模式，便于 Tauri 转发。 |

---

## 4. 前端功能模块（MVP：监控 + 诊断 + 刷写）

### 4.1 总线监控（Trace 视图）— MVP 监控
- [ ] 实时帧列表：ID、标准/扩展、DLC、数据(hex)、**时间戳(ms)**、出现次数、周期(ms)
- [ ] 发送面板：手动构造 CAN / **CAN-FD** 帧，支持循环发送 + 间隔
- [ ] 过滤：ID 单值 / 范围 / 掩码；冻结/清空/高亮
- [ ] 连接状态条：已连接 / Bus-Off / 错误被动（来自 `BusReady`/`BusError`）

### 4.2 UDS 诊断面板 — MVP 诊断
- [ ] 会话控制 `0x10`：下拉 Default/Programming/Extended
- [ ] ECU 复位 `0x11`：硬/软复位按钮
- [ ] TesterPresent `0x3E`：自动保活开关 + 周期配置
- [ ] 读/写 DID `0x22/0x2E`：DID 输入 + hex/ASCII 结果显示，**带 DID 名称/单位**（来自描述库）
- [ ] 读/清 DTC `0x19/0x14`：列表显示 **人类可读描述**（来自描述库）
- [ ] 安全访问 `0x27`：seed→key，算法可选（内置集）
- [ ] 例程控制 `0x31`：跑自检/校验
- [ ] 按地址读内存 `0x23` / IO 控制 `0x2F`（次要）

### 4.3 刷写面板 — MVP 刷写
- [ ] 固件选择：BIN（MVP）/ S19 / HEX（P1）
- [ ] 参数：target_id、memory_address、security_level、压缩/加密标志
- [ ] seed-key 算法选择（内置集）
- [ ] **实时进度**：块序号 / 总块 / 字节数（`FlashProgress`）
- [ ] 报告：耗时、块数、校验结果，可导出
- [ ] 擦除例程（P1-5）

### 4.4 配置面板
- [ ] 适配器类型：**PCAN（Windows）** 优先，通道选择（USB1…/PRO-FD）、波特率（BTR0BTR1）、**CAN-FD 开关 + 数据段波特率 + BRS/ESI**
- [ ] ISO-TP 参数（块大小、STmin）、UDS 超时（p2 / p2*）
- [ ] 多套配置保存/切换（车型/ECU）

### 4.5 日志与导出
- [ ] 全局收发 trace 日志（便于排错）
- [ ] 录制为 CSV（P2-1），可回放

---

## 5. 非功能需求（NFR）
- **实时性**：接收走独立任务 + 无锁队列；前端列表用虚拟滚动，高速下不卡顿（broadcast 高负载需 `try_recv` 或加大队列，监控 Lagged）。
- **跨平台**：MVP 聚焦 Windows(PCAN)；架构预留 Linux(SocketCAN) 与 macOS（无原生适配器，仅 SimBus）。
- **可扩展**：seed-key、DBC、新适配器以插件/配置接入。
- **稳定性**：长时运行不崩；硬件拔出/错误有 `BusError` 提示并自动断连。
- **可观测**：所有收发有 trace；命令失败有清晰错误码返回前端。

---

## 6. 分阶段交付计划

> MVP = 监控 + 诊断 + 刷写 一起交付（决策确认）。下面按依赖顺序排期，不是砍功能。

**阶段 0 — 后端打通（阻塞项，先于 UI 或并行）**
1. P0-1 PCAN 接收回路（补 FFI + 接收任务）
2. P0-2 迁移 `#[derive(Component)]`，`cargo check` 通过
3. P0-4 时间戳
4. 验证：SimBus + PCAN 都能在后端收到帧并 `emit_event`

**阶段 1 — 最小可用上位机（Tauri 骨架 + 监控）**
1. 新建 `app/`（src-tauri + Vue），`setup` 中 `BuildContext::build()`
2. commands/events 桥接（connect/send/frame 事件）
3. 4.1 监控视图（实时帧列表 + 发送面板 + 连接状态）
4. 4.4 配置面板（PCAN + FD 配置）

**阶段 2 — 诊断**
1. 4.2 UDS 面板全部指令 + 事件转发
2. P1-2 DTC/DID 描述库（先用内置库）

**阶段 3 — 刷写**
1. 4.3 刷写面板 + 进度事件
2. P1-3 seed-key 内置算法；P1-4 S19/HEX；P1-5 擦除例程（可并入阶段 3）

**阶段 4 — 完善（v2/v3）**
1. P0-3 FD ISO-TP（FD 帧走通诊断/刷写）
2. P1-1 过滤/统计；P2-1 录制/回放；P2-2 事件统一
3. DBC 解码、BLF/ASC、seed-key 插件化

---

## 7. 风险与待定
- **PCAN SDK 依赖**：需用户在 Windows 安装 PEAK PCAN-Basic 并放 `pcanbasic.dll`；MVP 开发阶段可用 SimBus 仿真避免硬件依赖。
- **FD ISO-TP 复杂度**：FD 路径的 CF/FC 与 escape 需严格对齐 ISO 15765-2:2016，建议用 Vector/真实 ECU 回归。
- **Tauri 与 DI 框架集成**：`BuildContext::build()` 在 `setup` 的阻塞/异步调用需验证；若与 Tauri 异步冲突，可改为显式 `invoke("init")` 命令触发。
- **描述库数据来源**：内置 DTC/DID 需用户提供其 ECU 规范，或先用通用/OEM 公开子集。

---

*本文档 v2 基于 `tx_di_can` 现有源码（adapter/plugin/isotp/uds/flash/frame）逐文件核实后修订，范围已由用户确认（Tauri + PCAN/Windows + CAN-FD + 完整 MVP）。下一步：进入阶段 0 后端改造与阶段 1 Tauri 骨架。*
