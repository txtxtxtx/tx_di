# CAN 诊断上位机 需求分析（tx_di_can）

> 目标读者：嵌入式诊断工程师 / 标定工程师 / 产线 / 项目负责人
> 目的：完整梳理"一个面向嵌入式诊断工程师的 CAN 诊断上位机到底需要哪些功能"，对照现有 `tx_di_can` 引擎（已逐文件核实）定位缺口，给出已确认范围的架构与分阶段交付计划。
> 状态：**v3 — 范围已确认（功能已补全、按真实引擎能力对齐）**

---

## 1. 已确认的范围决策

| 项 | 决策 | 影响 |
|----|------|------|
| 界面形态 | **Tauri**：Rust 后端 + Web(Vue3/TS) 桌面 App | 复用 `tx_di_can` 引擎，跨平台，与项目 examples(Vue/TS) 技术栈一致 |
| 目标硬件/系统 | **PCAN（PEAK PCAN-Basic）+ Windows** 优先 | 适配器优先级 PCAN > SimBus(仿真) > SocketCAN(Linux 留作后续) |
| 协议范围 | **UDS-on-CAN + CAN-FD（含 FD 版 ISO-TP）** | 必须实现 64 字节 FD ISO-TP 与 escape 序列 |
| MVP 范围 | **监控 + 诊断 + 刷写 完整 MVP 一起做** | 三大模块同批交付，不先砍功能 |

> 后续迭代（v2/v3）：Kvaser/Vector 适配器、DBC 解码、录制 BLF/ASC、seed-key 插件化、XCP 标定、ECU 仿真、离线分析、审计报表。

---

## 2. 角色与使用场景（补全）

| 角色 | 典型任务 | 上位机支撑 |
|------|----------|-----------|
| 嵌入式诊断工程师 | 调 ECU：读 DID、清 DTC、进编程会话刷固件、抓总线定位通信问题 | 监控 + UDS + 刷写 |
| 标定工程师 | 在线测量/标定参数（XCP on CAN）、看信号曲线 | 信号解码 +（后续）XCP |
| 产线/售后 | 批量刷写、读序列号/版本、跑自检例程、**操作留痕审计** | 刷写 + 自动化 + 报表 |
| 总线分析师 | 长时间监听、按信号解码、导出 trace 给 MATLAB/Vector 分析 | 监控 + DBC + 录制/回放 |
| 离线分析员 | 拿到录制的 BLF/ASC，事后分析、比对两次抓取差异 | 离线分析（后续） |
| 协议/测试工程师 | 自动跑诊断序列、CI 无头刷写、回归测试 | 自动化/脚本（后续） |

---

## 3. 系统架构（确认）

```
┌──────────────────────────────────────────────────────────┐
│  Tauri 桌面 App  (plugins/tx_di_can/app)                  │
│  ┌────────────┐  ┌────────────┐  ┌─────────────────────┐  │
│  │ Vue3 + TS  │  │ Pinia 状态 │  │ 视图：Trace/UDS/...  │  │
│  └────────────┘  └────────────┘  └─────────────────────┘  │
│        │ invoke / listen (Tauri IPC)                      │
│  ┌──────────────────────────────────────────────────┐    │
│  │ src-tauri (Rust)                                  │    │
│  │  Tauri Commands ──► tx_di_can 门面 (CanPlugin)    │    │
│  │  Event 转发: CanEvent ──► app.emit(...) ──► 前端   │    │
│  │  setup 中 BuildContext::build() 初始化 DI 容器     │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
                         │
   tx_di_can (协议引擎)  ──  tx-di-core (DI / App / Store / 事件)
                         │
   SimBus(仿真) / PCAN(Win) / SocketCAN(Linux) / Kvaser(占位)
```

**目录布局（建议新增成员）**
```
plugins/tx_di_can/app/src-tauri/  # Rust：commands.rs + events.rs + main.rs + tauri.conf.json
plugins/tx_di_can/app/src/       # Vue3：views/ stores/ api/
```

**前后端通信契约**
- 指令（前端→后端，封装 `CanPlugin` 静态 API）：`connect/disconnect`、`send_frame/send_fd_frame`、`read_did/write_did`、`read_dtc/clear_dtc`、`session_control/ecu_reset/tester_present`、`security_access/run_routine`、`read_memory`、`flash`。
- 事件（后端→前端）：`CanPlugin::on_event` 收到的 `CanEvent` 经 `app.emit("can://event", ev)` 转发；前端 `listen("can://event", ...)` 刷新界面。可用 `ts-rs` 自动生成 TS 类型镜像。
- 后端为全局单例（OnceLock），命令直接调用；DI 容器仅在 `setup` 调用一次 `BuildContext::build()`。

---

## 4. 后端能力映射（已核实，确保需求不悬空）

| 能力 | 后端现状 | 对应前端需求 |
|------|----------|--------------|
| UDS 0x10/0x11/0x14/0x19/0x22/0x23/0x27/0x2E/0x2F/0x31 | ✅ 已实现 | UDS 面板 |
| UDS 0x34/0x35/0x36/0x37 下载/上传原语 | ✅ 已实现（供刷写用） | 刷写面板；可复用做通用上传 |
| NRC 解析 + 0x78 pending 自动续等 | ✅ | 负响应友好提示 |
| 会话类型 Default/Programming/Extended | ✅ | 会话下拉 |
| 刷写 7 步流程 + 跳过 0xFF 块 + 进度/完成事件 | ✅ | 刷写面板 |
| 事件总线（13 类 CanEvent） | ✅ | 全部实时刷新 |
| CAN / CAN-FD 帧结构、SimBus/PCAN/SocketCAN 适配器 | ✅（PCAN 仅能发） | 监控/配置 |
| 配置：适配器/接口/波特率/FD 波特率/enable_fd/ISO-TP/p2 | ✅（TOML 静态） | 配置面板（需支持运行时改） |

---

## 5. 功能需求详述（FR，按模块，完整）

### 5.1 硬件接入与总线配置
- [ ] 适配器选择：PCAN（Windows 优先）/ SimBus（仿真）/ SocketCAN（Linux 后续）/ Kvaser（占位降级）
- [ ] 多通道：PCAN 多通道（USB1..8 / PRO-FD）分别配置与监控
- [ ] 仲裁段波特率 + **采样点**配置（当前缺）；CAN-FD：**数据段波特率 + BRS/ESI**（当前缺）
- [ ] 连接状态指示：已连接 / Bus-Off / 错误被动/主动；**错误计数器 TEC/REC** 显示（当前无）
- [ ] **Bus-Off 自动恢复**策略（当前无）
- [ ] 监听模式（listen-only，不发 ack）；终端电阻提示
- [ ] **热插拔检测**：PCAN 拔出/重插自动断连与重连
- [ ] 运行时改配置（当前只能从 TOML 加载，无法运行时改）

### 5.2 总线监控 / Trace 视图
- [ ] 实时帧列表（虚拟滚动）：ID、标准/扩展、RTR、DLC、数据(hex)、**时间戳(ms，当前恒0)**、出现次数、首见/末见时间、周期(ms)
- [ ] 数据显示切换：hex / 十进制 / 二进制 / ASCII
- [ ] 发送面板：手动构造并发送任意 CAN / **CAN-FD** 帧；循环发送 + 间隔；BRS/ESI 标志
- [ ] 过滤：ID 单值 / 范围 / 掩码；数据模式匹配；冻结/清空/高亮/着色
- [ ] 查找：在 Trace 中搜索 ID 或数据字节
- [ ] 列排序、行复制/导出选中行
- [ ] 统计：**总线负载率曲线**、错误帧计数、每 ID 帧率
- [ ] 信号曲线（配合 5.3）：选中信号实时绘图
- [ ] 多通道分栏/合并视图

### 5.3 信号级解码（DBC，后续 v2）
- [ ] 加载 DBC 数据库，把原始字节解码成命名信号（车速、转速、温度…）
- [ ] 信号数值表 + **实时曲线**；带单位、精度、因子/偏移、值表（枚举文本）
- [ ] 多路复用信号（multiplexor）解码
- [ ] 环境变量（environment variable）读写
- [ ] 基于 DBC 构造并发送报文（信号→字节）
- [ ] 报文发送触发条件（周期/on-change）

### 5.4 UDS 诊断服务（核心）
前端把 `uds.rs` 的每个 SID 做成可用面板；**所有响应需显示原始字节 + 解析值 + NRC 文本 + 耗时**。

| SID | 功能 | 前端形态 |
|-----|------|----------|
| 0x10 | 会话控制 | 下拉 Default/Programming/Extended + **当前会话状态指示** |
| 0x11 | ECU 复位 | 按钮（硬/软/钥匙断电） |
| 0x3E | TesterPresent | 自动保活开关 + 周期配置（默认 2000ms） |
| 0x22 | 读 DID | **支持多 DID 一次读**（当前后端单 DID，需扩展）；hex/ASCII/物理值显示；**带 DID 名称/单位**（描述库） |
| 0x2E | 写 DID | DID + 数据写入 |
| 0x19 | 读 DTC | 列表显示 **人类可读描述** + 状态位解析；**支持 sub-fn 0x01/0x02/0x0A**（快照/按状态/扩展，当前仅 0x02） |
| 0x14 | 清 DTC | 按钮（按组/全清） |
| 0x27 | 安全访问 | seed→key，**算法可选/可加载**（内置集 + 后续 .dll/.so）；显示 seed/key 字节 |
| 0x31 | 例程控制 | start/stop/result；跑自检/校验（含刷写校验例程） |
| 0x23 | 按地址读内存 | 地址 + 长度 + 地址/长度字段字节数 |
| 0x2F | IO 控制 | 输入输出控制 |
| 0x34~0x37 | 下载/上传 | 供刷写；也暴露为"通用上传（读 ECU 内存到文件）"工具 |

> 后续扩展 SID（当前后端未实现，列作规划）：0x28 通信控制、0x3D 写内存、0x83 时序参数、0x84 安全数据传输、0x86 事件响应、0x87 链路控制。

### 5.5 ISO-TP 传输层
- [x] 经典 ISO-TP（SF/FF/CF/FC，已有）
- [x] **CAN-FD ISO-TP**（64 字节 + escape 序列，已实现并通过单帧/多帧 round-trip 测试）
- [ ] 原始 ISO-TP 收发面板（绕过 UDS 传任意长数据）
- [ ] 流控可视化：block size、STmin、CF 序号、超时
- [ ] 多帧重组视图（看到完整重组后的 payload 与底层 CAN 帧对照）

### 5.6 刷写（Flash）
`flash.rs` 已有标准 7 步流程，缺口与增强：
- [ ] 文件格式：BIN（已有）/ **S19 / Intel HEX**（汽车固件常见，需新增解码器）
- [ ] **显式擦除例程**（0x31 0xFF00 或厂商擦除服务，当前仅 `erase_before_download` 隐式跳过 0xFF）
- [ ] **压缩/加密协商 UI**：当前后端 `0x34` 写死 `0x00`，需暴露参数并真正协商
- [ ] **seed-key 算法选择**：GUI 选内置算法（后续可加载插件）
- [ ] 参数：target_id、memory_address、security_level、session_type、verify_routine_id、block_size
- [ ] **实时进度**：块序号 / 总块 / 字节数 / 速率 / 预计剩余（FlashProgress 事件）
- [ ] **失败可视化**：后端失败时**补发 `FlashError` 事件**（枚举已有但未使用）
- [ ] 校验报告：耗时、块数、校验结果，可导出
- [ ] 中止/续传（断点续刷）、部分刷写（仅某地址区间）
- [ ] Bootloader 握手日志（每步请求/响应可查）

### 5.7 数据记录与回放（Record / Replay）
- [ ] 录制整段会话：CSV（MVP）/ **Vector ASC / BLF / PCAP / MDF**（后续）
- [ ] 触发录制（按 ID / 错误 / 手动起停）
- [ ] 回放：按原时序重发到总线复现问题；**速度调节（0.5x~10x）/ 循环 / 过滤**
- [ ] 回放时显示为独立通道，与实时流量区分

### 5.8 诊断数据库（描述库）
- [ ] DTC 描述库：把 `0xP1234` 映射成可读文本 + 状态位含义（内置通用集 + 外部 JSON/TOML 加载）
- [ ] DID 描述库：常用 DID（0xF190 VIN、0xF195 SW 版本、0xF18C 硬件版本…）带名称/单位/因子
- [ ] 后续：ODX/PDX、Vector CDD、OBD-II PID 库导入

### 5.9 自动化 / 脚本（后续）
- [ ] 宏录制/回放：把界面操作录成可重放序列
- [ ] 脚本（Python/JS）：串"进会话→安全访问→读 N 个 DID→出报告"
- [ ] 命令行 / headless 模式：CI 无头刷写、批量诊断
- [ ] 测试序列编辑器（步骤 + 预期响应断言）

### 5.10 ECU 仿真 / Restbus（后续）
- [ ] 仿真节点：按 DBC/描述库自动应答诊断请求，无需真实 ECU 即可联调上位机
- [ ] 网关/路由：多路 CAN 间报文转发与过滤
- [ ] 剩余总线仿真：按周期发送节点报文，维持总线活跃

### 5.11 标定 XCP/CCP on CAN（后续，标定工程师）
- [ ] A2L 文件加载，在线测量（DAQ）与标定（CAL）
- [ ] 参数曲线/示波

### 5.12 配置与项目管理
- [ ] 工程文件（.canproj）：保存适配器/ECU/DID/DTC/刷写参数整套配置
- [ ] 多套配置切换（不同车型/ECU）
- [ ] ISO-TP / UDS 超时（p2/p2*）、STmin、block size 可视化配置
- [ ] 最近会话/固件历史

### 5.13 离线分析（后续）
- [ ] 打开录制的 BLF/ASC/CSV，做信号分析、统计、查找
- [ ] 两次抓取差异比对（DID 值 diff、报文出现差异）

### 5.14 报表与审计（产线/售后）
- [ ] 刷写/诊断会话导出 PDF/HTML 报告（含时间、ECU、参数、结果、操作人）
- [ ] 操作留痕（审计日志），支持导出

### 5.15 国际化与权限
- [ ] 中英双语切换
- [ ] 产线模式：操作员/管理员权限分级（防误刷写）

---

## 6. 非功能需求（NFR）
- **实时性**：接收走独立任务 + 无锁队列；前端虚拟滚动，高速不卡顿（broadcast 高负载需 `try_recv` 或加大队列，监控 Lagged）。
- **跨平台**：MVP 聚焦 Windows(PCAN)；架构预留 Linux(SocketCAN) 与 macOS（仅 SimBus）。
- **可扩展**：seed-key、DBC、新适配器、脚本以插件/配置接入。
- **稳定性**：长时运行不崩；硬件拔出/错误有 `BusError` 并自动断连。
- **可观测**：所有收发有 trace 日志；命令失败有清晰错误码返回前端。
- **安全性**：安全访问/加密刷写参数不落明文日志；产线权限分级。
- **易用性**：常用操作 2 步内可达；默认值合理（p2=150ms/p2*=5000ms 等来自 `CanConfig` 默认）。

---

## 7. 后端改造与缺口（按优先级，先于或与 UI 同步）

| # | 缺口 | 现状（已核实） | 改造 |
|---|------|----------------|------|
| P0-1 | **PCAN 接收回路** | `adapter.rs` PCAN FFI 仅 `CAN_Write`，**无 `CAN_Read`/`CAN_ReadFD`** 且无后台接收任务；`close()` 有未用的 `tasks`/`running` 脚手架。 | 补 FFI + `open()` 后 `tokio::spawn` 接收任务推帧入 `tx`/`fd_tx`；`running` 标志优雅退出。 |
| P0-2 | **迁移新框架 API** | `plugin.rs` 用废弃的 `#[tx_comp(init)]`/`CompInit`/`InnerContext`/`BoxFuture`。 | 改 `#[derive(Component)] #[component(init, app_async_init, shutdown)]`，重接 `start_rx_loop`；目标 `cargo check` 通过。 |
| P0-3 | **FD ISO-TP** | 经典 SF/FF/CF/FC 已有；无 64 字节 FD 与 escape。 | `isotp.rs` 加 FD 路径（64B 数据帧、CF/FC、escape）；`IsoTpConfig` 加 `is_fd`；`plugin.rs` 已按 `enable_fd` 路由并转发 FD 帧到事件总线。✅ 已完成（2026-07-07） |
| P0-4 | **时间戳** | 接收帧时间戳恒为 0。 | 接收处填系统/硬件时间戳。 |
| P1-1 | **配置运行时可改** | 只能 TOML 静态加载。 | 暴露 `connect(config)` 命令，运行时重建适配器。 |
| P1-2 | **DTC/DID 描述库** | UDS 只给原始码/字节。 | 新增 `db/`：内置通用集 + 外部加载。 |
| P1-3 | **seed-key 算法可配置** | `key_fn` 是 Rust 闭包。 | 内置 2~3 种算法按 level 选；预留插件接口。 |
| P1-4 | **刷写文件格式/擦除/压缩加密** | 仅 BIN；擦除隐式；压缩加密写死 0x00；**失败不发 `FlashError`**。 | 加 S19/HEX 解码；显式擦除例程；暴露压缩/加密参数；补 `FlashError` 事件。 |
| P1-5 | **应用层过滤 + 统计** | 仅按 `rx_id` 过滤。 | 前端或后端 `subscribe` 包装过滤；暴露帧计数/负载率。 |
| P2-1 | **录制/回放** | 无导出。 | CSV（MVP）→ BLF/ASC（后续）；回放命令重发。 |
| P2-2 | **事件总线统一** | `event.rs` 独立全局总线，与 `tx-di-core` 事件体系不一致。 | 评估并入或对齐 gb28181 模式，便于 Tauri 转发。 |
| P2-3 | **多 DID 读 / DTC sub-fn** | `read_data` 单 DID；`read_dtc` 仅 0x02。 | 扩展 0x22 多 DID、0x19 的 0x01/0x0A。 |

---

## 8. 分阶段交付计划（MVP = 监控+诊断+刷写 同批）

**阶段 0 — 后端打通（阻塞项）**
1. P0-1 PCAN 接收回路（补 FFI + 接收任务）
2. P0-2 迁移 `#[derive(Component)]`，`cargo check` 通过
3. P0-4 时间戳
4. 验证：SimBus + PCAN 后端都能收帧并 `emit_event`

**阶段 1 — 最小可用上位机（Tauri 骨架 + 监控）**
1. 新建 `app/`（src-tauri + Vue），`setup` 中 `BuildContext::build()`
2. commands/events 桥接（connect/send/frame 事件）
3. 5.2 监控视图（实时帧列表 + 发送面板 + 连接状态 + 统计雏形）
4. 5.1/5.12 配置面板（PCAN + FD 配置，P1-1 运行时可改）

**阶段 2 — 诊断**
1. 5.4 UDS 面板全部 SID + 事件转发（含 NRC 文本/耗时）
2. P1-2 DTC/DID 描述库（内置集）
3. 5.5 ISO-TP 原始面板（经典）

**阶段 3 — 刷写**
1. 5.6 刷写面板 + 进度/错误事件
2. P1-3 seed-key 内置算法；P1-4 S19/HEX + 擦除 + 压缩/加密 UI；补 `FlashError`

**阶段 4 — 完善（v2/v3）**
1. ~~P0-3 FD ISO-TP（FD 帧走通诊断/刷写）~~ ✅ 已完成（2026-07-07）
2. P1-5 过滤/统计；P2-1 录制/回放；P2-2 事件统一；P2-3 多 DID/DTC sub-fn
3. 5.3 DBC 解码、5.7 高级录制格式、5.10 仿真、5.11 XCP、5.13 离线分析、5.14 审计报表

---

## 9. 验收标准（节选）
- **监控**：PCAN 真实硬件下能收发帧、时间戳非 0、负载率实时更新、过滤生效。
- **诊断**：0x10/0x22/0x19/0x27/0x3E 等全 SID 在真实/仿真 ECU 上得到正确解析响应与 NRC 提示。
- **刷写**：BIN/S19/HEX 固件在仿真 bootloader 上完成 7 步流程，进度实时、报告可导出、失败有 `FlashError` 提示。
- **稳定性**：连续监控 30 分钟无丢帧崩溃；PCAN 热拔插自动断连不 panic。

---

## 10. 风险与待定
- **PCAN SDK 依赖**：需 Windows 安装 PEAK PCAN-Basic 并放 `pcanbasic.dll`；MVP 开发可用 SimBus 仿真避开硬件依赖。
- **FD ISO-TP 复杂度**：FD 路径 CF/FC 与 escape 需严格对齐 ISO 15765-2:2016，用 Vector/真实 ECU 回归。
- **Tauri 与 DI 集成**：`BuildContext::build()` 在 `setup` 的调用需验证；若与 Tauri 异步冲突，可改为显式 `invoke("init")` 触发。
- **描述库数据来源**：内置 DTC/DID 需用户提供 ECU 规范，或先用通用/OEM 公开子集。
- **压缩/加密算法**：厂商私有，需用户提供依据再实现协商逻辑。

---

*本文档 v3 基于 `tx_di_can` 现有源码（adapter/plugin/isotp/uds/flash/frame/event/config）逐文件核实后修订，功能已补全并按真实引擎能力对齐。范围已由用户确认（Tauri + PCAN/Windows + CAN-FD + 完整 MVP）。下一步：阶段 0 后端改造 + 阶段 1 Tauri 骨架。*

---

## 11. 实施进度（2026-07-07）

### 已完成
- **P0-1 PCAN 收发**：`adapter.rs` 新增 `CAN_ReadFD` FFI + `start_recv()` 后台任务（同时收经典帧与 FD 帧、填时间戳、空队列 sleep 200µs）；`send_fd` 改走 `CAN_WriteFD`+`PcanMsgFd`（原 `CAN_Write` 仅 8 字节会截断 FD 载荷）。
- **P0-1 SocketCAN 收发**：设 `O_NONBLOCK` 并起非阻塞 `recvfrom` 后台任务，解析经典/FD 帧并打时间戳。
- **P0-2 插件迁移**：`plugin.rs` 从废弃 `CompInit` 迁移到 `#[derive(Component)]`（`init`/`app_async_init`/`shutdown`）。关键修正见下。
- **P0-4 时间戳**：`frame.rs` 新增 `now_micros()`，SimBus/PCAN/SocketCAN 接收帧均填真实时间戳。
- **运行时连接**：`INSTANCE` 改为 `RwLock<Option<Arc<CanPluginInner>>>`，新增 `connect`/`disconnect`/`is_connected`/`get_config`/`default_config`，支持 UI 切换适配器/比特率并在运行期重连（不再依赖 `BuildContext::build()`）。
- **事件可序列化**：`CanEvent` 派生 `Serialize`，供 Tauri `emit` 转发。
- **阶段 1 Tauri 骨架**：`app/src-tauri`（Cargo/tauri.conf.json/build.rs/capabilities/main.rs/commands.rs/events.rs）已 `cargo check` 零警告通过；新增为根 workspace 成员以复用 `tx_di_can`。
- **阶段 1 前端骨架**：`app/`（Vue3+TS+Vite）含 Trace/UDS/Flash/Config 四视图、`api/can.ts` 调用封装、`store.ts` 状态与事件分发。

### 关键修正（后续插件迁移务必注意）
- `#[derive(Component)]` 的生命周期回调（`init`/`shutdown`/`app_*`）是**模块级自由函数**，由宏生成 `self::init(self, store)` 等调用，故必须写成 `fn init(comp: &mut CanPlugin, ...)` / `fn shutdown(comp: &CanPlugin)`（**不可用 `&mut self`/`&self` 接收器**）；`app_async_init` 真实签名为 `async fn app_async_init(comp: Arc<CanPlugin>, app: Arc<App>)`（`app` 按值，非 `&Arc<App>`）；普通组件（Deps=()）需 `use tx_di_core::DepsTuple;`。

### 待办（后续迭代）
- 阶段 0 剩余：~~P0-3 FD ISO-TP~~ ✅ 已完成（2026-07-07，见 `isotp.rs` FD 路径）。
- 阶段 2/3：seed-key 算法 UI、S19/HEX + 压缩/加密、补 `FlashError` 真实触发、UDS 更多 SID 封装。
- 阶段 4：DBC 解码、录制/回放、过滤/统计、仿真/Restbus、XCP、离线分析、审计报表。
- 前端需 `npm install` 后 `npm run tauri dev`（依赖 Node 环境）；当前仅完成 Rust 侧编译验证。

