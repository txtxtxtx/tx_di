---
name: tx_admin 领域层 DDD 全面重构
overview: 对 tx_admin 的领域层做全面 DDD 重构：战略域重组（identity/system 分组）、强类型领域事件（各域独立事件枚举 + Event trait 对象分发）、RepositoryError 拆分到各域、补齐 job 领域层，并同步修改 app/infra/api/macros 层及测试。
todos:
  - id: shared-kernel
    content: 重构 shared 公共内核：迁出 UserStatus、移除巨型 DomainEvent、引入 Event trait 与泛型 AggregateRoot
    status: completed
---

## 产品概述

对 `examples/tx_admin/admin_domain` 进行一次性、彻底的 DDD 战略与战术重构，解决当前领域划分中存在的循环依赖、巨型事件/错误枚举、job 域缺失、auth 跨域依赖、password 定位不清等问题，使领域层边界清晰、依赖方向正确、可独立演进。

## 核心特性

1. **战略域重组**：将现有平铺的 11 个模块重组为 `identity`（身份与权限：user/role/menu/department/auth）、`system`（系统管理：config/dictionary/log/file）、`job`（新增任务调度域）、`shared`（真正不依赖具体域的公共内核，含安全工具）四大边界。
2. **强类型事件**：删除 shared 中巨型 `DomainEvent` 枚举，各域定义独立事件枚举；引入 `Event` 标记 trait（`Any + Send + Sync + Clone`）实现类型擦除与 `downcast` 分发；`DomainEventPublisher` 改用 `Vec<Arc<dyn Event>>`，`EventBus` 订阅者按具体类型分发。
3. **错误拆分**：删除 shared 中巨型 `RepositoryError` 枚举，各域定义自己的 `RepositoryError`（错误码语义保持不变，避免前端 i18n 回归）。
4. **job 领域层补齐**：在 `admin_domain` 新增 `job` 域（`Job`/`JobLog` 聚合根 + 不变量 + `JobRepository` trait + `JobService`），应用层改为依赖领域服务，infra 层实现 trait（适配 `tx_di_job` 插件或直接操作 toasty 模型）。
5. **循环依赖修复**：`UserStatus` 回归 user 域，`shared` 不再 `use` 任何具体域。
6. **auth 与 password 定位**：auth 归入 identity 并改为依赖 `UserRepository`（而非跨域 `UserService`）；password 哈希工具迁入 `shared/security`。

## 约束

- `admin_domain` 领域层不得依赖 `tx_di_job`、`tx_di_toasty` 等基础设施插件（保持依赖倒置）。
- 现有前端错误码（`RepositoryError` 各变体的 `#[err(code, msg)]`）语义与数值必须保持稳定。
- 保持现有测试可编译通过（或同步更新测试代码）。
- 不引入新的架构模式，沿用现有 `model/repository/service` 三层 + `#[derive(Component)]` + `as_trait` 注册约定。

## 技术栈

- 语言/框架：Rust + `tx-di`（`#[derive(Component)]`、`as_trait`、生命周期回调）
- 宏：`admin_macros`（`#[derive(AggregateRoot)]`，需改造支持自定义事件类型）
- 事件：`std::any::Any` + 自定义 `Event` trait 做类型擦除与 `downcast_ref` 分发
- ORM：`toasty`（infra 层仓储实现）
- 密码：`argon2`（Argon2id，迁入 `shared/security`）

## 实现方案

### 总体策略

采用**目录移动 + 路径重写 + 类型替换**三阶段推进，每阶段保持可编译（或阶段性可编译），避免一次性大爆炸导致难以定位错误。

1. **先修共享内核**：删除 `shared` 对 `user` 的反向依赖（`UserStatus` 迁回 user），让 `shared` 成为纯公共内核。
2. **再拆事件**：引入 `Event` trait + 各域事件枚举，改造 `AggregateRoot` 宏、`DomainEventPublisher`、`EventBus`、各 aggregate 与订阅点。
3. **再拆错误**：各域定义 `RepositoryError`，同步改 service/infra 与测试。
4. **再做战略重组**：移动目录到 `identity`/`system`/`job`/`shared/security`，更新所有 `use` 路径。
5. **最后补 job 域**：新增 domain 聚合/仓储 trait/服务，改造 app/infra 层。

### 关键技术决策

#### 1. 强类型事件：`Event` trait + `Arc<dyn Event>` 下转型

`shared/model/event.rs` 新增：

```rust
pub trait Event: Any + Send + Sync + Clone {
    fn event_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}
```

各域定义独立事件枚举（如 `user::model::event::UserEvent`），为枚举实现 `Event`。`DomainEventPublisher` 改为：

```rust
pub trait DomainEventPublisher: Send + Sync {
    fn publish(&self, events: Vec<Arc<dyn Event>>);
}
```

`EventBus::subscribe` 改为泛型订阅（或订阅 `Arc<dyn Event>` 后由调用方 `downcast_ref` 分发）。为降低迁移成本，可提供一个便捷方法 `EventBus::on::<E: Event>(&self, handler: Fn(E))`，内部用 `TypeId` 分发表路由。

**权衡**：`Arc<dyn Event>` 相比 `enum DomainEvent` 增加了堆分配（每个事件一个 `Arc`），但换来各域事件解耦、可扩展（新增域事件不改公共文件）。对于管理后台的低频事件发布场景，性能开销可忽略。

#### 2. AggregateRoot 宏改造

`admin_macros` 的 `#[derive(AggregateRoot)]` 需要支持自定义事件类型。由于各域事件类型不同，宏无法硬编码 `crate::shared::model::DomainEvent`。方案：

- 宏接受可选属性 `#[aggregate_root(event = UserEvent)]` 指定事件类型；或
- 宏生成泛型化 impl，事件类型通过结构体字段 `events: Vec<E>` 推断，但 Rust 宏无法稳定从字段类型推断泛型。

**推荐方案**：宏增加属性 `#[aggregate_root(event = path::to::Event)]`，`events` 字段类型固定为 `Vec<E>`（`E` 实现 `Event`）。生成：

```rust
impl Entity for X { type Id = u64; ... }
impl AggregateRoot<path::to::Event> for X {
    fn events(&self) -> &[path::to::Event];
    fn clear_events(&mut self);
    fn add_event(&mut self, event: path::to::Event);
}
```

同时 `AggregateRoot` trait 泛型化为 `AggregateRoot<E: Event>`（或保留非泛型 trait + 每域事件实现 `Event` 后统一用 `Arc<dyn Event>` 收集）。为保持 `publish_events` 逻辑简单，聚合根内部仍存 `Vec<E>`，`events()` 返回 `&[E]`，发布时 map 成 `Arc<dyn Event>`。

#### 3. RepositoryError 拆分

删除 `shared/repository.rs` 的巨型枚举，保留 `db_err`（`tx_error::log_err` 重导出）。各域 `repository/mod.rs` 定义自己的错误枚举（如 `user::repository::UserRepositoryError`），变体与错误码沿用原 `RepositoryError` 中对应项（如 `DatabaseUser=10001`、`NotFoundUser=10101`、`DuplicateUsername=10201`、`ValidationUserStatus=10301`）。

`#[derive(CodeMsg)]` + `#[err(code, msg)]` 的 `#[err]` 前缀（当前 `#[err("REPOSITORY")]`）可保留为 `"REPOSITORY"` 以维持前端错误编码稳定，或按域改为 `"USER"/"ROLE"` 等（需同步前端）。**为降低回归风险，建议保留 `"REPOSITORY"` 前缀不变**，仅将枚举按域物理拆分，错误码数值不变。

跨域共享的错误（如 `ValidationToken`、`ValidationLogin` 属于 auth）需归属到对应域，避免删除后无处安放。

#### 4. job 域补齐

`admin_domain/job/` 新增：

- `model/aggregate.rs`：`Job`（id/name/status/handler_name/handler_param/cron_expression/retry_count/retry_interval/monitor_timeout/audit/soft_delete）与 `JobLog` 聚合根，封装不变量（如「运行中任务可修改但不能删除」「cron 表达式合法」「重试次数非负」）。
- `model/event.rs`：`JobEvent`（JobCreated/JobUpdated/JobDeleted/JobStatusChanged/JobLogCreated/JobLogFinished）。
- `model/value_object.rs`：`JobStatus`、`ExecutionStatus`、`JobQuery`、`JobLogQuery`。
- `repository/mod.rs`：`JobRepository` trait（create/update/delete/find_by_id/find_page/change_status/find_log_page/clean_logs 等）。
- `service/mod.rs`：`JobService`，持有 `Arc<dyn JobRepository>`，封装创建/更新/启停/触发/日志用例。
- `error.rs`：`JobRepositoryError`（JobNotFound 等）。

infra 层新增 `admin_infra/job/repository.rs` 实现 `JobRepository` trait，内部适配 `tx_di_job` 的 `JobRepository`（或直接操作 toasty 模型，需评估是否新增 `SysJob`/`SysJobLog` toasty 模型）。为减少与插件耦合，**推荐在 admin_infra 直接操作 toasty 模型**（对齐 user/role 等域的既有模式），`tx_di_job` 的 `JobPlugin` 仅保留调度执行职责，`JobRepository` 数据访问下沉到 admin_infra。

app 层 `JobAppService` 改为依赖 `Arc<JobService>`（或 `Arc<dyn JobRepository>`），`run_job` 仍需 `JobPlugin` 执行（执行是基础设施能力，保留在 app 层通过注入 `JobPlugin` 调用，或抽象为 `JobExecutor` trait 由 infra 实现）。

#### 5. 目录与路径重组

```
admin_domain/src/
├── identity/
│   ├── mod.rs
│   ├── user/ role/ menu/ department/   (原目录迁移)
│   └── auth/                            (迁移 + 改为依赖 UserRepository)
├── system/
│   ├── mod.rs
│   ├── config/ dictionary/ log/ file/  (原目录迁移)
├── job/
│   └── ...                              (新增)
├── shared/
│   ├── mod.rs
│   ├── model/ (entity.rs, aggregate_root.rs, audit.rs, event.rs, value_object.rs)
│   ├── event_publisher.rs
│   └── security/
│       └── password.rs                  (原 password 模块迁移)
└── lib.rs                               (更新模块导出 + re-export 兼容旧路径)
```

为降低调用方（app/infra/api）改造成本，`lib.rs` 提供**兼容 re-export**（如 `pub use identity::user as user;`、`pub use system::config as config;`），使旧路径 `admin_domain::user::...` 短期仍可用，同时鼓励逐步迁移到新路径。但注意：re-export 会造成路径二义性，建议重构时**同步更新所有调用方路径**，不保留长期兼容层。

### 数据流

```
聚合根方法（add_event(E)）
  → AppService 事务提交后 publish_events
    → aggregate.events() → map 成 Arc<dyn Event>
      → DomainEventPublisher::publish(Vec<Arc<dyn Event>>)
        → EventBus 按 TypeId 路由 → 各订阅者 downcast_ref 分发
```

## 实现细节（执行要点）

- **性能**：事件发布为进程内低频操作，`Arc<dyn Event>` 额外堆分配可接受；`EventBus` 订阅用 `HashMap<TypeId, Vec<Subscriber>>` 做 O(1) 路由，避免 O(n) 线性匹配。
- **日志**：沿用 `tx_error::log_err`（`db_err`）和 `tracing`，事件分发失败时记录 warn 不中断主流程；避免打印大 payload。
- **爆破半径**：分阶段提交，每阶段可编译；保留错误码数值与 `"REPOSITORY"` 前缀防前端回归；目录移动用 `git mv` 保留历史；所有 `use` 路径用编译器错误逐一定位修正。
- **兼容性**：`AggregateRoot` 宏是 breaking change，需同步更新全部 8 个聚合根的 `#[derive]` 属性与 `events` 字段类型、以及各域 `model/tests.rs` 中事件断言。

## 架构设计

### 系统架构

```
┌─────────────────────────────────────────────────────────┐
│  admin_domain (领域层，不依赖 infra 插件)                 │
│  ├── shared  (Event trait / Entity / AggregateRoot<E> /  │
│  │            AuditFields / DomainEventPublisher /       │
│  │            security::password)                        │
│  ├── identity (user/role/menu/department/auth)           │
│  ├── system   (config/dictionary/log/file)               │
│  └── job      (Job/JobLog 聚合 + JobRepository trait)    │
└──────────────┬──────────────────────────────────────────┘
               │ 依赖倒置（trait 在 domain，实现在 infra）
┌──────────────▼──────────────────────────────────────────┐
│  admin_infra (Toasty 仓储实现 + JobExecutor 实现)          │
└──────────────┬──────────────────────────────────────────┘
               │
┌──────────────▼──────────────────────────────────────────┐
│  admin_app (应用服务编排 + EventBus + JobAppService)       │
└──────────────┬──────────────────────────────────────────┘
               │
┌──────────────▼──────────────────────────────────────────┐
│  admin_api (HTTP/gRPC 接口 + 事件订阅示例)                 │
└─────────────────────────────────────────────────────────┘
```

依赖方向严格单向：api → app → domain ← infra（app/infra 都依赖 domain，infra 实现 domain trait）。

### 模块职责

- **shared**：纯公共内核，不依赖任何具体域。承载 `Event` trait、`Entity`/`AggregateRoot<E>`、`AuditFields`、`DomainEventPublisher`、密码哈希工具。
- **identity**：身份与权限限界上下文，含 user/role/menu/department 聚合与 auth 认证服务（依赖 `UserRepository`）。
- **system**：系统管理支撑域，轻建模（config/dictionary/log/file，去除过度的事件仪式）。
- **job**：任务调度域，聚合根封装任务不变量，仓储 trait 与调度执行抽象分离。
- **事件**：各域事件枚举实现 `Event` trait，进程内 `EventBus` 按 `TypeId` 路由。

## 目录结构

以下为重构涉及的关键文件（[NEW]=新增，[MODIFY]=修改，[MOVE]=目录移动）：

```
admin_domain/
├── src/
│   ├── lib.rs                                      # [MODIFY] 更新模块导出路径
│   ├── shared/
│   │   ├── mod.rs                                  # [MODIFY] 移除 user 依赖
│   │   ├── event_publisher.rs                      # [MODIFY] publish 改为 Vec<Arc<dyn Event>>
│   │   ├── model/
│   │   │   ├── mod.rs                              # [MODIFY] 拆分，移除巨型 DomainEvent
│   │   │   ├── entity.rs                           # [NEW] Entity trait
│   │   │   ├── aggregate_root.rs                   # [NEW] AggregateRoot<E> trait
│   │   │   ├── event.rs                            # [NEW] Event trait
│   │   │   ├── audit.rs                            # [NEW] AuditFields（从 mod.rs 迁出）
│   │   │   └── value_object.rs                     # [MODIFY] 移除 SessionEctData 的 user 依赖
│   │   └── security/
│   │       ├── mod.rs                              # [NEW]
│   │       └── password.rs                         # [MOVE] 原 password/mod.rs
│   ├── identity/
│   │   ├── mod.rs                                  # [NEW]
│   │   ├── user/                                   # [MOVE] 原 user/，event 改独立枚举
│   │   │   ├── model/event.rs                      # [MODIFY] UserEvent 枚举
│   │   │   ├── model/aggregate.rs                  # [MODIFY] events: Vec<UserEvent>
│   │   │   ├── repository/mod.rs                   # [MODIFY] 定义 UserRepositoryError
│   │   │   └── service/mod.rs                      # [MODIFY] 用 UserRepositoryError
│   │   ├── role/                                   # [MOVE] 同 user 改造
│   │   ├── menu/                                   # [MOVE] 同 user 改造
│   │   ├── department/                             # [MOVE] 同 user 改造
│   │   └── auth/
│   │       ├── mod.rs                              # [MOVE]
│   │       ├── error.rs                            # [MODIFY] AuthError 保持
│   │       └── service.rs                          # [MODIFY] 依赖 UserRepository
│   ├── system/
│   │   ├── mod.rs                                  # [NEW]
│   │   ├── config/ dictionary/ log/ file/          # [MOVE] 同 user 改造（事件/错误拆分）
│   └── job/
│       ├── mod.rs                                  # [NEW]
│       ├── model/
│       │   ├── mod.rs                              # [NEW]
│       │   ├── aggregate.rs                        # [NEW] Job/JobLog 聚合根
│       │   ├── event.rs                            # [NEW] JobEvent 枚举
│       │   └── value_object.rs                     # [NEW] JobStatus/ExecutionStatus/查询
│       ├── repository/mod.rs                       # [NEW] JobRepository trait
│       ├── service/mod.rs                          # [NEW] JobService
│       └── error.rs                                # [NEW] JobRepositoryError

admin_macros/
└── src/lib.rs                                      # [MODIFY] AggregateRoot 宏支持 event 属性

admin_infra/
├── src/
│   ├── lib.rs                                      # [MODIFY] 模块路径 + job 模块
│   ├── plugin.rs                                   # [MODIFY] job 模型注册（如需要）
│   ├── user/repository.rs                          # [MODIFY] 用 UserRepositoryError
│   ├── role/ menu/ department/ config/ dictionary/ log/ file/  # [MODIFY] 错误类型
│   └── job/
│       ├── mod.rs                                  # [NEW]
│       └── repository.rs                           # [NEW] JobRepository trait 实现

admin_app/
├── src/
│   ├── event_bus.rs                                # [MODIFY] Arc<dyn Event> + TypeId 路由
│   ├── user/app_service.rs                         # [MODIFY] 事件发布 + 路径
│   ├── role/ menu/ department/ config/ dictionary/ log/ file/  # [MODIFY] 路径 + 错误
│   └── job/
│       ├── app_service.rs                          # [MODIFY] 依赖 JobService
│       └── dto.rs                                  # [MODIFY] 从 domain Job 转 proto

admin_api/
└── src/plugin.rs                                   # [MODIFY] 事件订阅改 downcast 分发
```

## 关键代码结构

### Event trait（shared/model/event.rs）

```rust
pub trait Event: std::any::Any + Send + Sync + Clone {
    fn event_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn std::any::Any;
}
```

### DomainEventPublisher（shared/event_publisher.rs）

```rust
pub trait DomainEventPublisher: Send + Sync {
    fn publish(&self, events: Vec<std::sync::Arc<dyn Event>>);
}
```

### EventBus 订阅 API（admin_app/event_bus.rs）

```rust
impl EventBus {
    pub fn on<E: Event + 'static>(&self, handler: impl Fn(E) + Send + Sync + 'static);
    pub fn publish(&self, events: Vec<Arc<dyn Event>>);
}
```

内部用 `HashMap<TypeId, Vec<Arc<dyn Fn(Arc<dyn Event>) + Send + Sync>>>` 做路由分发。

### AggregateRoot 宏属性

```rust
#[derive(AggregateRoot)]
#[aggregate_root(event = crate::identity::user::model::event::UserEvent)]
pub struct User {
    pub id: u64,
    // ...
    events: Vec<crate::identity::user::model::event::UserEvent>,
}
```

宏生成泛型 `AggregateRoot<E: Event>` 的 impl，`events()` 返回 `&[E]`，`add_event(event: E)`，`clear_events()`。

## 推荐扩展

### SubAgent

- **code-explorer**
- 用途：在重构实施阶段，若出现大量 `use` 路径失效，用于快速定位某类型/符号的所有引用点（跨 admin_domain/admin_app/admin_infra/admin_api 四个 crate），减少手工 grep 遗漏。
- 预期结果：输出精确的受影响文件清单与行号，指导路径重写与错误类型替换。

### Skill

- **rust-ddd-test-generator**
- 用途：在完成领域层重构后（事件/错误/聚合/job 域就位），为各域生成或更新测试套件，覆盖聚合不变量、值对象、领域服务、仓储 trait（mockall）与事件分发，确保重构后生产可用。
- 预期结果：产出覆盖各聚合根业务规则、各域 `RepositoryError` 语义、`EventBus` 分发、`JobService` 用例的测试代码，配合 `cargo test` 全绿。