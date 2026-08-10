---
name: tx_admin-架构设计说明书
overview: 以系统架构师视角，为 tx_admin（Rust + DDD + tx_di DI 框架的后台管理系统示例）编写一份完整的《软件架构设计说明书》，覆盖从需求空间到 SA（软件架构）模型的完整流程，包含设计、实现、构建组装、部署四大阶段，并采用 4+1 视图与架构风格分析。
todos:
  - id: verify-code-facts
    content: 使用 [skill:lsp-code-analysis] 核对 admin_api/app/domain/infra 的模块结构与 DI 注册链，确认文档引用的真实路径
    status: completed
  - id: draft-requirements
    content: 撰写文档引言与需求空间分析：13个业务域、功能/质量属性/约束空间，映射到实际模块
    status: completed
    dependencies:
      - verify-code-facts
  - id: draft-styles-sa
    content: 撰写架构风格分析（分层/管道-过滤器/事件驱动/插件/微服务 + C4）与 SA 模型确立、ADRs
    status: completed
    dependencies:
      - draft-requirements
  - id: draft-stages
    content: 撰写设计/实现/构建组装/部署四大阶段，含 DI 初始化顺序与微服务扩展
    status: completed
    dependencies:
      - draft-styles-sa
  - id: draft-views
    content: 撰写 4+1 视图（逻辑/进程/开发/物理/场景）全部用 Mermaid 图表达
    status: completed
    dependencies:
      - draft-stages
  - id: draft-quality-close
    content: 撰写质量属性改进章节（M1/M2/L4/L6 等架构级建议）并整合成完整 09 文档、校验格式
    status: completed
    dependencies:
      - draft-views
---

## 用户需求

以系统架构师身份，为 `examples/tx_admin` 后台管理系统补充一份**完整的软件架构设计文档**，覆盖从需求空间到 SA（软件架构）模型的完整生命周期。

## 产品概述

文档需系统化阐述 tx_admin 的整体架构设计，使其从"实现代码"层面上升到"架构蓝图"层面，可作为系统演进、评审、培训与实施的标准依据。

## 核心特性

- **完整生命周期**：需求空间 → SA 模型
- **四大阶段**：设计阶段、实现阶段、构建组装阶段、部署阶段
- **4+1 视图**：逻辑视图、进程视图（开发视图）、物理视图、场景视图（用例）
- **架构风格**：分层、管道-过滤器、事件驱动、微服务/单服务多实例、C4 模型等多视角分析
- **专业架构师视角**：含 ADR、质量属性、架构权衡、已知问题改进方向
- **纯文档交付**：新增至 `docs/`，不改动任何代码

## 交付物

- 新增 `docs/09-软件架构设计说明书-从需求到SA模型.md`
- 遵循既有文档风格（编号延续、标题层级、表格、Mermaid 图、ADR）

## 技术方案概述

本文档为**纯文档交付**，不修改任何代码。作为系统架构师，基于对 tx_admin 实际代码结构（已验证的真实 crate/模块路径）撰写架构设计说明书。文档严格遵循软件架构设计的标准方法论（Kruchten 4+1 视图、C4 模型、SEI 质量属性场景）。

### 文档核心结构（对齐用户四大硬性要求）

1. **从需求空间到 SA 模型**

- 需求空间：业务域分析（认证/用户/角色/菜单/部门/权限/配置/字典/文件/日志/任务/监控/工具 13 个业务域）、功能需求空间（RBAC、双协议、审计）、质量需求空间（安全、性能、可用性、可扩展性）、约束空间（技术栈约束、Send+Sync、DepsTuple≤16）
- 架构空间：候选架构生成 → 权衡分析（tactic-based）→ SA 模型确立（最终四层 DDD 分层 + DI 组合装配）

2. **四大阶段**（每一阶段均结合真实代码路径）

- 设计阶段：逻辑架构、物理架构、数据架构、接口契约（proto/HTTP DTO）
- 实现阶段：模块实现规范、命名约定、DI 生命周期、质量门禁（cargo clippy/test/fmt）
- 构建组装阶段：Cargo workspace 依赖图、DI 组件注册与拓扑排序、`App::ins_run()` 初始化顺序（DbInitPlugin i32::MAX-200 → AdminPlugin i32::MAX-100 → WebPlugin 默认）、linkme 分布式切片注册机制
- 部署阶段：单体部署、config.toml 配置、以及 docs/06 的微服务多实例扩展方案（Nginx + r-nacos + Redis/S3 无状态化）

3. **4+1 视图**（Kruchten 方法）

- 逻辑视图（开发视角）：DDD 四层 + 依赖方向、11 个 AppService、10 个领域子域、Repository 抽象
- 进程视图（运行视角）：axum HTTP + tonic gRPC 双进程模型、tokio 异步运行时、OperateLogLayer mpsc 异步通道、async_run 并发后台任务、领域事件总线
- 开发视图（实现视角）：Cargo workspace 模块划分、crate 依赖图、#[tx_comp] DI 注册、linkme 切片
- 物理视图（部署视角）：节点拓扑、进程绑定、config.toml 环境配置、微服务多实例拓扑
- 场景视图：登录认证、创建用户、操作日志、定时任务 4 个典型端到端场景（用 Mermaid 时序/流程图）

4. **架构风格分析**

- 主风格：DDD 分层架构（接口/应用/领域/基础设施），依赖倒置
- 辅助风格：管道-过滤器（操作日志/认证中间件链）、事件驱动（DomainEvent + event_bus）、微内核/插件（tx_di 插件体系）、依赖注入（编译期 DI）、微服务（单服务多实例演进路径）
- C4 模型映射（Context → Container → Component → Code）
- 每个风格给出适用性评估与权衡（trade-off）

### 关键技术点（基于已验证事实）

- DI 生命周期链：build → inner_init → init → async_init → async_run → shutdown（逆拓扑序）
- 初始化排序常量：tx_di_log(i32::MIN)、Config(i32::MIN+1)、toasty(i32::MIN+2)、file(i32::MIN+3)、sip(10000)、axum(i32::MAX)
- 接口层：HTTP 12 模块 + gRPC 13 服务 + AdminPlugin；权限 ensure_permission + gRPC AuthLayer
- 质量属性与已知问题映射（M1 领域事件未消费、M2/L7 Job 全量分页、L4 链路追踪缺失、L6 API 版本、配置硬编码、gRPC 401）→ 给出架构层面改进建议

### 文档渲染技术

- 全部架构图使用 Mermaid（flowchart / sequenceDiagram / stateDiagram / C4），语言标注明确
- 表格组织：业务域表、依赖表、视图对照表、质量属性表、ADR 表
- 编号延续 `09-`，遵循 docs/ 既有风格

## 架构设计（文档目录蓝图）

```
docs/09-软件架构设计说明书-从需求到SA模型.md
├── 一、引言（目的/范围/读者/术语/参考文档）
├── 二、需求空间分析（业务域/功能/质量属性/约束）
├── 三、架构风格分析（分层/管道-过滤器/事件驱动/插件/微服务 + C4 + 权衡）
├── 四、SA模型确立（架构决策、ADRs、候选比较）
├── 五、设计阶段（逻辑/进程/物理/数据架构、接口契约）
├── 六、实现阶段（模块规范、命名约定、DI生命周期、质量门禁）
├── 七、构建组装阶段（workspace依赖、DI注册/拓扑、ins_run初始化）
├── 八、部署阶段（单体配置、微服务多实例、环境矩阵）
├── 九、4+1视图（逻辑/进程/开发/物理/场景，Mermaid图）
├── 十、质量属性与已知问题架构级改进
└── 十一、架构演进路线与附录（ADR汇总、术语表）
```

## 目录结构

纯文档交付，仅新增 1 个文件：

```
docs/
└── 09-软件架构设计说明书-从需求到SA模型.md  # [NEW] 完整软件架构设计文档，覆盖需求空间→SA模型、四大阶段、4+1视图、架构风格；基于真实代码路径撰写，全部图用 Mermaid，遵循既有文档风格
```

## Agent 扩展

### Skill

- **lsp-code-analysis**: 在撰写实现阶段与构建组装阶段时，借助 LSP 语义分析精确核对 DI 组件注册、AppService 依赖注入链路、模块引用关系（定义/引用/实现），确保文档中引用的 crate/模块/类型路径与实际代码完全一致，避免虚构。
- **find-skills**: 若在撰写过程中发现需要额外的文档生成或架构辅助能力（如生成架构图辅助材料），用于检索是否存在可安装的相关技能；预期产出为确认当前交付链路已足够完整，无需额外引入。