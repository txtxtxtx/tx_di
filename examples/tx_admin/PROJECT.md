# tx_admin 项目详细说明文档
```shell
cargo build -p admin_api --release
```
> **生成时间**: 2026-06-29（基于最新代码更新）
> **适用范围**: Bug 修复、功能开发、架构演进指引

---

# ⚠️ 当前问题与待优化清单

> 以下问题为代码走查后汇总，按严重程度排列，建议优先处理高危项。

## 🔴 高危问题（影响安全 / 稳定性）

| # | 问题 | 位置 | 影响 |
|---|------|------|------|
| ~~P1~~ | ~~**配置路径硬编码**~~ | ~~`admin_api/src/main.rs:32`~~ | ✅ **已确认为测试环境，暂不处理** |
| ~~P2~~ | ~~**密码明文比较**~~ | ~~`admin_app/src/auth/app_service.rs`~~ | ✅ **误判：密码已使用 Argon2id 安全哈希**（见 `admin_domain/src/password/mod.rs`） |
| P3 | **gRPC 拦截器存在潜在 401 响应体问题** | `admin_api/src/interfaces/grpc/auth_interceptor.rs` | ⚠️ **见下方详细审查说明** |

### P3 详细审查结论 — gRPC `auth_interceptor.rs`

经过全面代码审查，整体逻辑**安全可靠**，但存在以下**已知技术问题**：

#### ✅ 正确的部分
- `OPEN_METHODS` 白名单明确仅放行 `Login`，其余全部强制鉴权——策略正确
- 使用 `StpUtil::get_login_id(&token)` 委托 sa-token 完成 token 合法性验证（会话存在性、有效期），逻辑正确
- `ensure_grpc_permission()` 中 admin 角色直接放行，普通角色走 `check_permission`，RBAC 逻辑清晰
- `GrpcLoginId` 注入 request extensions，服务方法通过 `get_login_id()` 统一提取，无直接暴露 token 的问题
- 已有完整的 `extract_bearer_token` 单元测试覆盖正常、缺失、格式错误三种情形

#### ⚠️ 建议改进的问题 忽略

| # | 问题 | 风险级别 | 说明 |
|---|------|----------|------|
| P3-a | **401 响应体为空** | 低 | `ResBody::default()` 会产生空 body，gRPC 客户端解析可能抛异常而非收到可读错误码，建议改为返回标准 gRPC `Status::unauthenticated` | 
| P3-b | **`OPEN_METHODS` 硬编码字符串** | 低 | 方法路径以字面量维护，若 proto 包名变更容易漏改，建议提取为常量并加注释说明对应的 proto 定义 |
| P3-c | **`GrpcToken` 存入 extensions 后标记 `#[allow(dead_code)]`** | 低 | 当前 token 注入进 extensions 但没有地方消费，若不需要可移除以减少冗余 |
| P3-d | **`AuthLayer::new()` 可改为 `Default` trait 实现** | 极低 | 语义上是无参构造，实现 `Default` 更符合 Rust 惯例 |

## 🟡 中等问题（影响功能完整性）

| # | 问题 | 位置 | 说明 |
|---|------|------|------|
| M1 | **领域事件未消费** | 全局 | `DomainEvent` 被聚合根收集但从未发布/处理，事件驱动架构形同虚设 |
| M2 | **Job 列表全量加载** | `admin_app/src/job/app_service.rs:94` | `get_all_jobs()` 拉全量到内存再筛选分页，数据量大时 OOM 风险 |

## 🟢 低优先级优化

| # | 问题 | 说明 |
|---|------|------|
| L1 | 部分聚合根未使用 `#[derive(AggregateRoot)]` 宏 | `DictType`, `DictData`, `Config`, `File`, `Log` 手动实现 |
| L2 | `get_user_info` 中重复查询用户 | `auth/app_service.rs` 调用 `get_user()` 两次 |
| L3 | gRPC 服务错误处理不统一 | 各 gRPC service 的 `into_status()` 转换方式不一致 |
| L4 | 缺乏 OpenTelemetry / 链路追踪 | 分布式场景无法追踪跨服务请求 |
| L5 | 前端 `web/` 为编译产物 | 8749 个文件占用仓库空间，建议 `.gitignore` |
| L6 | 缺少 API 版本管理策略 | 所有路由无版本前缀，未来演进困难 |
| L7 | `admin_infra` 已接真实 DB，但 Job 分页仍走全量内存筛选 | DB 分页能力未充分利用 |

---

## 1. 项目概述

`tx_admin` 是一个基于 **Rust** 的后台管理系统示例项目，展示了 `tx_di` 依赖注入框架在实际业务场景中的应用。系统采用 **领域驱动设计（DDD）分层架构**，支持 **HTTP（axum）和 gRPC（tonic）双协议**，已通过 `admin_infra`（toasty ORM）接入真实数据库（SQLite / PostgreSQL / MySQL）。

### 1.1 核心业务域

| 业务域 | 说明 |
|--------|------|
| **认证 (Auth)** | 登录 / 登出 / 获取用户信息（sa-token 认证） |
| **用户 (User)** | 用户 CRUD、密码修改、角色/部门绑定 |
| **角色 (Role)** | 角色 CRUD、菜单权限分配 |
| **菜单 (Menu)** | 菜单树管理（目录/页面/按钮三级） |
| **部门 (Department)** | 部门树管理（支持层级结构） |
| **权限 (Permission)** | 基于角色的权限查询与校验 |
| **配置 (Config)** | 系统配置键值对管理 |
| **字典 (Dictionary)** | 字典类型 + 字典数据（如性别、状态等枚举） |
| **文件 (File)** | 文件上传记录管理（本地/S3 双后端） |
| **日志 (Log)** | 操作日志 + 登录日志 |
| **定时任务 (Job)** | Cron 任务管理、执行日志、手动触发 |
| **监控 (Monitor)** | 系统监控（占位，待实现） |
| **工具 (Tool)** | 系统工具（占位，待实现） |

### 1.2 技术栈

| 层次 | 技术选型 |
|------|---------|
| HTTP 框架 | axum 0.8 |
| gRPC 框架 | tonic + prost |
| 异步运行时 | tokio |
| ORM | toasty（via `tx_di_toasty`） |
| 认证 | sa-token-rust（via `tx_di_sa_token`） |
| 序列化 | serde + serde_json |
| 时间处理 | jiff（Timestamp） |
| ID 生成 | 雪花算法（自研，见 `tx_common::id`） |
| 错误处理 | tx_error（统一 AppError 体系） |
| 依赖注入 | tx-di-core（编译期 DI 框架） |
| 定时任务 | tx_di_job（Cron 调度器） |
| 文件存储 | tx_di_file（本地/S3） |
| Proto 代码生成 | tonic-build + protoc_bin_vendored |

---

## 2. 整体架构

### 2.1 分层架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        admin_api (接口层)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────────┐│
│  │  HTTP API    │  │  gRPC Service│  │  AdminPlugin (DI组件)   ││
│  │  (axum)      │  │  (tonic)     │  │  - 操作日志 Layer       ││
│  │  12 个模块    │  │  13 个模块    │  │  - 注册 HTTP/gRPC 路由  ││
│  └──────┬───────┘  └──────┬───────┘  └─────────────────────────┘│
│         │                 │                                      │
│         └────────┬────────┘                                      │
│                  ▼                                                │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │         DI 容器注入（DiComp<T> / App::inject）               ││
│  │  各 AppService 通过 #[tx_comp] 注册，handler 直接 inject     ││
│  └──────────────────────────┬──────────────────────────────────┘│
├─────────────────────────────┼───────────────────────────────────┤
│                        admin_app (应用层)                         │
│  ┌──────────────────────────┴──────────────────────────────────┐│
│  │  11 个 AppService（编排领域服务，DTO 转换，事务协调）         ││
│  │  AuthAppService / UserAppService / RoleAppService / ...     ││
│  │  + JobAppService（基于 tx_di_job）                          ││
│  └──────────────────────────┬──────────────────────────────────┘│
├─────────────────────────────┼───────────────────────────────────┤
│                       admin_domain (领域层)                       │
│  ┌──────────────────────────┴──────────────────────────────────┐│
│  │  10 个子域：user / role / menu / department / permission /   ││
│  │            config / dictionary / file / log / shared         ││
│  │  每个子域包含：                                               ││
│  │    model/ (aggregate + value_object + event + tests)         ││
│  │    repository/ (trait 定义)                                   ││
│  │    service/ (领域服务)                                        ││
│  └─────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────┤
│                      admin_infra (基础设施层)                      │
│  Toasty ORM Repository 实现 + DbInitPlugin（种子数据）           │
│  支持 SQLite / PostgreSQL / MySQL                                │
├─────────────────────────────────────────────────────────────────┤
│  admin_proto (传输层)  │  admin_macros (派生宏)                   │
│  Proto 定义 + 代码生成  │  AggregateRoot derive                    │
├─────────────────────────────────────────────────────────────────┤
│                    公共库 / 框架层                                │
│  tx-di-core │ tx_di_axum │ tx_di_log │ tx_error │ tx_common    │
│  tx_di_sa_token │ tx_di_toasty │ tx_di_job │ tx_di_file        │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 依赖关系（Crate 依赖图）

```
admin_api
  ├── admin_proto       (Proto DTO 定义)
  ├── admin_app         (应用服务)
  │     ├── admin_domain (领域模型 + 领域服务)
  │     │     ├── admin_macros (AggregateRoot 派生宏)
  │     │     ├── tx_error     (统一错误)
  │     │     └── tx_common    (雪花ID、分页、API响应)
  │     ├── admin_proto
  │     ├── tx_di_job    (定时任务框架)
  │     ├── tx_di_toasty (ORM 插件)
  │     ├── tx_error
  │     └── tx_common
  ├── admin_infra       (DB Repository 实现)
  ├── tx-di-core        (DI 框架核心)
  ├── tx_di_axum        (axum Web 插件)
  ├── tx_di_log         (日志插件)
  ├── tx_di_sa_token    (认证插件)
  ├── tx_di_file        (文件存储插件)
  ├── tx_di_job         (定时任务插件)
  ├── tx_error
  └── tx_common
```

---

## 3. 各 Crate 详解

### 3.1 `admin_api` — 接口层

**路径**: `examples/tx_admin/admin_api/`
**职责**: 启动应用、注册 HTTP/gRPC 路由、认证鉴权、操作日志记录

#### 3.1.1 目录结构

```
admin_api/src/
├── main.rs              # 入口：构建 DI 容器，启动 App，注册 Job Handler
├── plugin.rs            # AdminPlugin：操作日志 Layer + HTTP/gRPC 路由注册
├── auth.rs              # ensure_permission() 权限检查工具
├── error.rs             # ApiErr 错误类型（HTTP 响应错误）
├── operate_log.rs       # OperateLogLayer：HTTP 请求日志中间件
└── interfaces/
    ├── api/             # HTTP 处理器（12 个模块）
    │   ├── mod.rs       # open_router() + router() 路由注册
    │   ├── auth_api.rs
    │   ├── user_api.rs
    │   ├── role_api.rs
    │   ├── menu_api.rs
    │   ├── dept_api.rs
    │   ├── config_api.rs
    │   ├── dict_api.rs
    │   ├── log_api.rs
    │   ├── file_api.rs
    │   ├── monitor_api.rs  # 占位
    │   ├── tool_api.rs     # 占位
    │   └── job_api.rs
    └── grpc/            # gRPC 服务实现（13 个模块）
        ├── auth_interceptor.rs  # gRPC 认证拦截器（Tower Layer）
        ├── auth_service.rs
        ├── user_service.rs
        ├── role_service.rs
        ├── menu_service.rs
        ├── dept_service.rs
        ├── config_service.rs
        ├── dict_service.rs
        ├── log_service.rs
        ├── file_service.rs
        ├── monitor_service.rs
        ├── tool_service.rs
        ├── job_service.rs
        └── err.rs
```

#### 3.1.2 启动流程

```rust
// main.rs 核心流程
let config_path = r"C:\...\config\config.toml";   // ⚠️ 硬编码路径
let app = BuildContext::new(Some(config_path)).build()?;  // 1. 构建 DI 容器
let app = app.ins_run().await?;                            // 2. 异步初始化所有组件
// 3. 注册内置 Job Handler（noop / echo）
Ok(app.waiting_exit().await)                               // 4. 等待退出信号
```

**`ins_run()` 内部流程**:
1. `App::init()` — 按 `init_sort()` 排序，依次调用各组件的同步 `init()`
2. `App::async_init()` — 按排序依次调用各组件的异步 `async_init()`
3. `App::comp_run()` — 并行启动所有带 `async_run` 的组件

初始化排序（关键顺序）：
```
DbInitPlugin (i32::MAX - 200)  → 注册 toasty 模型 + 种子数据
AdminPlugin  (i32::MAX - 100)  → 注册路由 + 启动 gRPC
WebPlugin    (默认)             → 启动 HTTP 服务器
```

#### 3.1.3 路由架构

```
公开路由 (无需认证)
  POST /api/auth/login
  POST /api/auth/refresh
  GET  /files/**           # 静态文件访问

受保护路由 (需要 sa-token)
  POST /api/auth/logout
  POST /api/auth/user-info
  CRUD /api/user/**
  CRUD /api/role/**
  CRUD /api/menu/**
  CRUD /api/dept/**
  CRUD /api/config/**
  CRUD /api/dict/**
  CRUD /api/log/**
  CRUD /api/file/**
  CRUD /api/job/**
  GET  /api/monitor/**
  GET  /api/tool/**
```

#### 3.1.4 操作日志中间件

```rust
// OperateLogLayer (sort=15, 在 api_log sort=10 之后)
// 异步通道 mpsc::channel，容量 OPERATE_LOG_CHANNEL_CAP
// 每次 HTTP 请求完成后：user_id / user_name / method / uri / status / latency_ms
// → CreateOperateLogRequest → OperateLogAppService::create_log()
```

#### 3.1.5 gRPC 服务（已启用）

gRPC 监听 `0.0.0.0:50051`（硬编码），通过 `AuthLayer` Tower 中间件做认证拦截，注册了 13 个 gRPC 服务：Auth / User / Role / Menu / Dept / Config / Dict / Log / File / Monitor / Tool / Job / JobLog。

---

### 3.2 `admin_app` — 应用层

**路径**: `examples/tx_admin/admin_app/`
**职责**: 编排领域服务、DTO 转换、事务协调

#### 3.2.1 目录结构

```
admin_app/src/
├── lib.rs               # 模块导出
├── empty_string.rs      # 空字符串辅助工具
├── auth/
│   ├── app_service.rs   # AuthAppService（#[tx_comp]）
│   ├── dto.rs
│   └── session_service.rs  # SessionService（会话管理）
├── user/                # UserAppService（#[tx_comp]）
├── role/                # RoleAppService（#[tx_comp]）
├── menu/                # MenuAppService（#[tx_comp]）
├── department/          # DepartmentAppService（#[tx_comp]）
├── config/              # ConfigAppService（#[tx_comp]）
├── dictionary/          # DictTypeAppService + DictDataAppService（#[tx_comp]）
├── log/                 # OperateLogAppService + LoginLogAppService（#[tx_comp]）
├── file/                # FileAppService（#[tx_comp]）
└── job/                 # JobAppService（#[tx_comp]，基于 tx_di_job）
```

#### 3.2.2 DI 注册模式

所有 AppService 均通过 `#[tx_comp]` 宏注册到 DI 容器，依赖自动注入：

```rust
#[tx_comp]
pub struct UserAppService {
    user_service: Arc<UserService>,  // 领域服务，自动从 DI 注入
}
```

Handler 中直接使用 `DiComp<T>` 提取器（无需手动调用 `services::get()`）：

```rust
async fn create_user(
    DiComp(svc): DiComp<UserAppService>,  // DI 自动注入
    Json(req): Json<CreateUserRequest>,
) -> Result<ApiR<UserResponse>, ApiErr> {
    // ...
}
```

> **与旧版 PROJECT.md 的重大变化**: 已从 `OnceLock<Arc<Svc>>` 手动组装模式迁移到 DI 框架原生注入模式，`services.rs` 文件已不存在。

#### 3.2.3 JobAppService 特殊说明

`JobAppService` 不依赖 `admin_domain` 领域层，而是直接使用 `tx_di_job` 框架提供的 `JobRepository` / `InfrustJob` / `InfrustJobLog` 等类型，并通过 `JobPlugin` 执行任务。

**已知问题**: `get_job_page` / `get_job_log_page` 调用 `get_all_jobs()` 拉全量数据到内存后再筛选分页，需改为数据库分页查询。

---

### 3.3 `admin_domain` — 领域层

（与旧文档基本一致，见原始聚合根、Repository Trait、领域服务汇总表，此处不重复）

**新增说明**: `admin_domain` 不依赖任何基础设施，领域服务通过 Repository Trait 抽象与 DB 解耦。

---

### 3.4 `admin_infra` — 基础设施层（已实现）

**路径**: `examples/tx_admin/admin_infra/`
**状态**: ✅ 已完整实现，使用 **toasty ORM** 连接真实数据库

#### 3.4.1 目录结构

```
admin_infra/src/
├── lib.rs               # 导出 + register_models()
├── plugin.rs            # DbInitPlugin：模型注册 + 种子数据初始化
├── seed.rs              # 种子数据（初始管理员、菜单、角色等）
├── common/mod.rs        # 共享工具（分页查询等）
├── user/                # UserRepository 实现
├── role/                # RoleRepository 实现
├── menu/                # MenuRepository 实现
├── department/          # DepartmentRepository 实现
├── config/              # ConfigRepository 实现
├── dictionary/          # DictTypeRepository + DictDataRepository 实现
├── file/                # FileRepository + FileConfigRepository 实现
└── log/                 # OperateLogRepository + LoginLogRepository 实现
```

#### 3.4.2 DbInitPlugin

- `inner_init`：注册所有 toasty 模型（在 DB 连接前，sort = `i32::MAX - 200`）
- `async_init`：当 `auto_schema = true` 时，检测空数据库并执行种子数据初始化

#### 3.4.3 数据库配置

```toml
[toasty_config]
auto_schema = false          # false = 用户自管表结构；true = 自动建表 + 种子数据
database_url = "sqlite:examples/tx_admin/data/tx_admin.db"
max_pool_size = 10
```

---

### 3.5 `admin_proto` — 传输层

**已添加模块**: job.proto（JobService / JobLogService），monitor.proto，tool.proto

---

### 3.6 其他说明

| Crate | 功能 |
|-------|------|
| `admin_macros` | `#[derive(AggregateRoot)]` 派生宏 |
| `tx_di_sa_token` | sa-token 认证（JWT-like，支持 Redis/内存会话） |
| `tx_di_file` | 文件上传（本地 / S3 双后端），`FileConfig` 配置化 |
| `tx_di_job` | Cron 调度器，支持 Handler 注册 / 执行日志 / 手动触发 |
| `tx_di_toasty` | toasty ORM 封装，多数据库支持 |

---

## 4. 依赖注入框架

### 4.1 组件生命周期

```
1. #[tx_comp] 注册 → COMPONENT_REGISTRY (linkme 分布式切片)
2. BuildContext::new() → 拓扑排序 → 按序注册工厂
3. BuildContext::build() → 创建 App
4. App::ins_run() →
   a. inner_init()   — 同步内部初始化（注册模型等）
   b. init()         — 同步初始化
   c. async_init()   — 异步初始化（按 init_sort 排序）
   d. comp_run()     — 并行运行所有 async_run 组件
5. App::waiting_exit() → 等待 Ctrl+C / SIGTERM → graceful shutdown
```

### 4.2 本项目 DI 使用现状

- ✅ 基础设施（HTTP、gRPC、日志、DB、认证、文件、任务）全部通过 DI 管理
- ✅ 所有 AppService 通过 `#[tx_comp]` 注册，Handler 直接 `inject()` 使用
- ✅ 依赖注入链路完整：AppService → DomainService → Repository → DB

---

## 5. 数据流详解

### 5.1 典型请求流程（"创建用户"）

```
HTTP POST /api/user/
  │
  ▼
user_api::create_user()          ← 接口层（axum handler）
  │ 1. DiComp<UserAppService> 从 DI 注入
  │ 2. ensure_permission("user:create") 权限校验
  │ 3. 反序列化 JSON → CreateUserRequest
  │ 4. 转换为 CreateUserCommand
  │ 5. 调用 svc.create_user(cmd, creator)
  │
  ▼
UserAppService::create_user()    ← 应用层
  │ 1. 检查邮箱/手机号唯一性
  │ 2. 调用 UserService::create_user()
  │ 3. 设置可选字段 + 分配角色/部门
  │ 4. 转换为 UserResponse
  │
  ▼
UserService::create_user()       ← 领域层
  │ 1. 检查用户名唯一性
  │ 2. 生成雪花 ID
  │ 3. User::create() → 创建聚合根 + 添加领域事件
  │ 4. user_repo.insert()
  │
  ▼
admin_infra::UserRepository      ← 基础设施层（toasty ORM）
  │ SQL INSERT → SQLite/PG/MySQL
  │
  ▼
返回 User → UserResponse → ApiR::success() → JSON
```

### 5.2 认证流程（sa-token）

```
POST /api/auth/login (公开路由)
  │
  ▼
AuthAppService::login()
  │ 1. UserService::get_by_username()
  │ 2. 检查用户状态
  │ 3. 验证密码（⚠️ 当前为明文比较，需改为 argon2/bcrypt）
  │ 4. SessionService::create_session() → StpUtil::login()
  │    生成 sa-token，存入会话存储
  │ 5. 构建 LoginResponse (token, user_info, permissions)
  │
  ▼
后续请求携带 token (Authorization header)
SaTokenLayer → SaCheckLoginLayer → 验证 token → 注入 login_id

Handler 中：
  StpUtil::get_login_id_as_string() → 获取当前用户 ID
  ensure_permission("xxx:yyy")       → 权限校验
```

---

## 6. 构建与运行

### 6.1 前置条件

- Rust 1.75+
- protoc（通过 `protoc_bin_vendored` crate 自动提供，无需手动安装）
- SQLite（默认，零配置）或 PostgreSQL / MySQL

### 6.2 构建命令

```bash
# 构建整个项目
cargo build

# 仅构建 admin_api
cargo build -p admin_api

# 运行（需先修正 main.rs 中的配置路径，或改为从环境变量读取）
cargo run -p admin_api

# 运行测试
cargo test -p admin_app

# 运行特定测试
cargo test -p admin_app --test workflow_test
```

### 6.3 配置文件

```toml
# config/config.toml
[web_config]
port = 8888
host = "::"

[toasty_config]
database_url = "sqlite:examples/tx_admin/data/tx_admin.db"
auto_schema = false

[sa_token_config]
timeout = 86400
token_name = "Authorization"

[job_config]
enabled = true
poll_interval_secs = 1

[file_config]
backend = "local"  # or "s3"
base_path = "./uploads"
```

> ⚠️ **问题**: 配置路径在 `main.rs` 中硬编码，建议改为通过环境变量 `CONFIG_PATH` 或命令行参数传入。

---

## 7. 已知问题与待办事项

（详见文档顶部"⚠️ 当前问题与待优化清单"）

### 7.1 最高优先级修复

**P1 修复配置路径硬编码**:
```rust
// main.rs 建议改为：
let config_path = std::env::var("CONFIG_PATH")
    .unwrap_or_else(|_| "config/config.toml".to_string());
let app = BuildContext::new(Some(&config_path)).build()?;
```

**P2 修复密码明文**:
```rust
// 接入 argon2 crate
use argon2::{Argon2, PasswordHash, PasswordVerifier};
// 验证：Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
// 创建用户时：Argon2::default().hash_password(password.as_bytes(), &salt)
```

---

## 8. 功能开发指引

（新增模块、修改 API、接入数据库的步骤与之前相同，参考原有 Checklist）

---

## 9. 架构决策记录 (ADR)

### ADR-001: 已从 Mock 仓库迁移到 Toasty ORM
**状态**: Completed  
**变更**: `admin_infra` 已完整实现所有 Repository Trait，通过 toasty ORM 连接真实数据库。

### ADR-002: Proto 生成的类型作为传输 DTO
**状态**: Accepted（继续沿用）

### ADR-003: 已从 OnceLock 迁移到 DI 原生注入
**状态**: Completed  
**变更**: 删除了 `services.rs`，所有 AppService 通过 `#[tx_comp]` 注册，Handler 使用 `DiComp<T>` 提取器。

### ADR-004: sa-token 作为认证方案
**状态**: Accepted  
**决策**: 使用 `tx_di_sa_token` 插件（基于 sa-token-rust），支持 token 自动续期、多端登录控制等。

---

## 10. 快速参考

### 10.1 关键文件速查

| 需求 | 文件路径 |
|------|---------|
| 启动入口 | `admin_api/src/main.rs` |
| 路由注册 | `admin_api/src/interfaces/api/mod.rs` |
| 权限检查 | `admin_api/src/auth.rs` |
| 操作日志中间件 | `admin_api/src/operate_log.rs` |
| 统一响应 | `tx_common/src/api_r.rs` |
| 分页封装 | `tx_common/src/page.rs` |
| ID 生成 | `tx_common/src/id.rs` |
| 错误定义 | `tx_error/src/error.rs` |
| 领域事件 | `admin_domain/src/shared/model/mod.rs` |
| 审计字段 | `admin_domain/src/shared/model/mod.rs` (AuditFields) |
| 软删除 | `admin_domain/src/shared/model/value_object.rs` (DeletedStatus) |
| DB 初始化 | `admin_infra/src/plugin.rs` |
| 种子数据 | `admin_infra/src/seed.rs` |

### 10.2 命名约定

| 类型 | 命名规则 | 示例 |
|------|---------|------|
| 聚合根 | 名词 | `User`, `Role`, `Menu` |
| 值对象 | 名词 + Query/Status/Type | `UserQuery`, `UserStatus` |
| Repository trait | 名词 + Repository | `UserRepository` |
| 领域服务 | 名词 + Service | `UserService` |
| 应用服务 | 名词 + AppService | `UserAppService` |
| Command DTO | 动词 + 名词 + Command | `CreateUserCommand` |
| Response DTO | 名词 + Response | `UserResponse` |
| HTTP Handler | 动词（小写下划线） | `create_user`, `get_role` |
| Proto 消息 | 动词 + 名词 + Request/Response | `CreateUserRequest` |

---


---

# 微服务改造方案（单服务多实例）

> **目标**: 将 `tx_admin` 整体作为一个微服务，注册到 r-nacos，支持多实例水平扩展、故障自动转移
> **详细文档**: [docs/06-微服务改造方案-单服务多实例.md](docs/06-微服务改造方案-单服务多实例.md)

---

## 一、改造目标

| 目标 | 说明 |
|------|------|
| **服务注册** | 每个实例启动自动注册到 r-nacos，下线自动摘除 |
| **配置中心** | 配置迁移到 r-nacos，支持热更新 |
| **水平扩展** | 无状态化后启动 N 个实例，负载均衡分担流量 |
| **故障转移** | 健康检查失败自动摘除，流量转发到健康实例 |
| **自动重启** | Docker `restart: unless-stopped` / K8s `livenessProbe` |

## 二、架构

```
  Nginx (负载均衡)
      ├── tx_admin 实例1 (注册到 r-nacos)
      ├── tx_admin 实例2 (注册到 r-nacos)
      └── tx_admin 实例3 (注册到 r-nacos)
            │
      r-nacos (服务注册 + 配置中心)
            │
  共享基础设施: PostgreSQL + Redis + S3
```

## 三、关键改造点

### 3.1 服务注册（P0）

新建 `admin_api/src/nacos.rs`，启动时调用 r-nacos HTTP API 注册实例，定期发送心跳，关闭时注销。

环境变量控制：
- `NACOS_SERVER_ADDR` — r-nacos 地址
- `NACOS_NAMESPACE` — 命名空间（dev/prod）
- `SERVICE_IP` / `SERVICE_HTTP_PORT` — 实例地址

### 3.2 无状态化（水平扩展前提）

| 改造项 | 方案 |
|--------|------|
| 会话共享 | sa-token 会话存 Redis（多实例间 session 一致） |
| 文件上传 | 使用 S3 兼容存储（`tx_di_file` 已支持） |
| 定时任务 | Redis 分布式锁，防止多实例重复执行 |

### 3.3 健康检查

新增 `GET /health`（存活）、`GET /health/ready`（就绪）、`GET /health/live`（存活）端点，r-nacos 定期探测，失败自动摘除实例。

### 3.4 自动重启

- **Docker**: `restart: unless-stopped` + `healthcheck`
- **Kubernetes**: `livenessProbe` + `readinessProbe`，K8s 自动重启失败 Pod

## 四、实施路线图

| 阶段 | 内容 | 优先级 |
|------|------|--------|
| P0 | 服务注册到 r-nacos + `/health` 端点 | 高 |
| P1 | sa-token Redis 化 + 配置中心迁移 | 高 |
| P2 | 文件存储改 S3 + Job 分布式锁 | 中 |
| P3 | Docker Compose 多实例验证 | 中 |
| P4 | Kubernetes 生产部署 | 低 |

## 五、验证

```bash
# 启动 3 个实例
docker-compose up --scale tx_admin=3

# 故障转移：停止一个实例，确认流量不再转发到它
docker-compose stop tx_admin_1

# 自动重启：杀掉进程，确认 Docker 自动重启
docker kill tx_admin_2
docker ps | grep tx_admin_2  # 应看到容器已重启
```

