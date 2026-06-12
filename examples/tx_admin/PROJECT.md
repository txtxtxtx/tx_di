# tx_admin 项目详细说明文档

> **生成时间**: 2026-06-12  
> **适用范围**: Bug 修复、功能开发、架构演进指引

---

## 1. 项目概述

`tx_admin` 是一个基于 **Rust** 的后台管理系统示例项目，展示了 `tx_di` 依赖注入框架在实际业务场景中的应用。系统采用 **领域驱动设计（DDD）分层架构**，支持 **HTTP（axum）和 gRPC（tonic）双协议**，当前使用 **Mock 内存仓库** 作为数据存储层（尚未接入真实数据库）。

### 1.1 核心业务域

| 业务域 | 说明 |
|--------|------|
| **认证 (Auth)** | 登录 / 登出 / 获取用户信息 |
| **用户 (User)** | 用户 CRUD、密码修改、角色/部门绑定 |
| **角色 (Role)** | 角色 CRUD、菜单权限分配 |
| **菜单 (Menu)** | 菜单树管理（目录/页面/按钮三级） |
| **部门 (Department)** | 部门树管理（支持层级结构） |
| **权限 (Permission)** | 基于角色的权限查询与校验 |
| **配置 (Config)** | 系统配置键值对管理 |
| **字典 (Dictionary)** | 字典类型 + 字典数据（如性别、状态等枚举） |
| **文件 (File)** | 文件上传记录管理 |
| **日志 (Log)** | 操作日志 + 登录日志 |

### 1.2 技术栈

| 层次 | 技术选型 |
|------|---------|
| HTTP 框架 | axum 0.x |
| gRPC 框架 | tonic + prost |
| 异步运行时 | tokio |
| 序列化 | serde + serde_json |
| 时间处理 | jiff（Timestamp） |
| ID 生成 | 雪花算法（自研，见 `tx_common::id`） |
| 错误处理 | tx_error（统一 AppError 体系） |
| 依赖注入 | tx-di-core（编译期 DI 框架） |
| Proto 代码生成 | tonic-build + protoc_bin_vendored |

---

## 2. 整体架构

### 2.1 分层架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        admin_api (接口层)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────────┐│
│  │  HTTP API    │  │  gRPC Service│  │  AdminPlugin (DI组件)   ││
│  │  (axum)      │  │  (tonic)     │  │  - 初始化 Mock 仓库     ││
│  │  10 个模块    │  │  10 个模块    │  │  - 注册 HTTP 路由       ││
│  └──────┬───────┘  └──────┬───────┘  └─────────────────────────┘│
│         │                 │                                      │
│         └────────┬────────┘                                      │
│                  ▼                                                │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              services.rs (全局服务注册表)                     ││
│  │  OnceLock<Arc<Svc>> → 12 个 AppService 的集合                ││
│  └──────────────────────────┬──────────────────────────────────┘│
├─────────────────────────────┼───────────────────────────────────┤
│                        admin_app (应用层)                         │
│  ┌──────────────────────────┴──────────────────────────────────┐│
│  │  12 个 AppService（编排领域服务，DTO 转换，事务协调）         ││
│  │  AuthAppService / UserAppService / RoleAppService / ...     ││
│  └──────────────────────────┬──────────────────────────────────┘│
│                              │                                   │
│  ┌──────────────────────────┴──────────────────────────────────┐│
│  │  mock/ (Mock 仓库实现，内存 HashMap + RwLock)                ││
│  │  12 个 MockXxxRepository                                      ││
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
│  admin_proto (传输层)  │  admin_macros (派生宏)  │  admin_infra   │
│  Proto 定义 + 代码生成  │  AggregateRoot derive   │  (预留基础设施) │
├─────────────────────────────────────────────────────────────────┤
│                    公共库 / 框架层                                │
│  tx-di-core │ tx_di_axum │ tx_di_log │ tx_error │ tx_common    │
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
  │     ├── tx_error
  │     └── tx_common
  ├── tx-di-core        (DI 框架核心)
  ├── tx_di_axum        (axum Web 插件)
  ├── tx_di_log         (日志插件)
  ├── tx_error
  └── tx_common
```

---

## 3. 各 Crate 详解

### 3.1 `admin_api` — 接口层

**路径**: `examples/tx_admin/admin_api/`  
**职责**: 启动应用、注册 HTTP/gRPC 路由、接收请求并转发到应用层

#### 3.1.1 目录结构

```
admin_api/src/
├── main.rs              # 入口：构建 DI 容器，启动 App
├── plugin.rs            # AdminPlugin：DI 组件，初始化 Mock 仓库 + 注册路由
├── services.rs          # 全局服务注册表（OnceLock<Arc<Svc>>）
└── interfaces/
    ├── mod.rs
    ├── api/             # HTTP 处理器（10 个模块）
    │   ├── mod.rs       # router() 注册所有子路由
    │   ├── auth_api.rs
    │   ├── user_api.rs
    │   ├── role_api.rs
    │   ├── menu_api.rs
    │   ├── dept_api.rs
    │   ├── permission_api.rs
    │   ├── config_api.rs
    │   ├── dict_api.rs
    │   ├── log_api.rs
    │   └── file_api.rs
    ├── dto/mod.rs       # 复用 tx_common 的 ApiR/ApiRes/Page
    └── grpc/            # gRPC 服务实现（10 个模块，当前已注释）
        ├── mod.rs
        ├── auth_service.rs
        ├── user_service.rs
        └── ...
```

#### 3.1.2 启动流程

```rust
// main.rs 核心流程
let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
let app = BuildContext::new(Some(config_path)).build()?;  // 1. 构建 DI 容器
let app = app.ins_run().await?;                            // 2. 异步初始化所有组件
Ok(app.waiting_exit().await)                               // 3. 等待退出信号
```

**`ins_run()` 内部流程**:
1. `App::init()` — 按 `init_sort()` 排序，依次调用各组件的同步 `init()`
2. `App::async_init()` — 按排序依次调用各组件的异步 `async_init()`
3. `App::comp_run()` — 并行启动所有带 `async_run` 的组件

#### 3.1.3 `AdminPlugin` 组件

```rust
#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> AppResult<()> {
        services::init_services();              // 初始化 Mock 仓库 + 领域服务 + 应用服务
        WebPlugin::add_router(api::router(ctx)); // 注册 HTTP 路由到 WebPlugin
        Ok(())
    }
    fn init_sort() -> i32 { i32::MAX - 100 }   // 确保在 WebPlugin 之后初始化
}
```

#### 3.1.4 `services.rs` — 全局服务注册表

**核心结构**:
```rust
pub struct Svc {
    pub auth:       AuthAppService,
    pub user:       UserAppService,
    pub role:       RoleAppService,
    pub menu:       MenuAppService,
    pub dept:       DepartmentAppService,
    pub perm:       PermissionAppService,
    pub config:     ConfigAppService,
    pub dict_type:  DictTypeAppService,
    pub dict_data:  DictDataAppService,
    pub oper_log:   OperateLogAppService,
    pub login_log:  LoginLogAppService,
    pub file:       FileAppService,
}
static SERVICES: OnceLock<Arc<Svc>> = OnceLock::new();
```

**组装逻辑** (`init_services()`):
1. 创建 12 个 Mock 仓库实例
2. 用仓库创建 11 个领域服务
3. 用领域服务创建 12 个应用服务
4. 存入 `OnceLock`，全局单例

**访问方式**: `services::get()` 返回 `&'static Arc<Svc>`

#### 3.1.5 HTTP 路由表

| 路径前缀 | 模块 | 方法 |
|----------|------|------|
| `POST /api/auth/login` | auth_api | 登录 |
| `POST /api/auth/user-info` | auth_api | 获取用户信息 |
| `POST /api/auth/logout` | auth_api | 登出 |
| `POST /api/user/` | user_api | 创建用户 |
| `GET /api/user/{user_id}` | user_api | 获取用户 |
| `PUT /api/user/{user_id}` | user_api | 更新用户 |
| `DELETE /api/user/{user_id}` | user_api | 删除用户 |
| `POST /api/user/list` | user_api | 用户分页列表 |
| `POST /api/user/change_password` | user_api | 修改密码 |
| `POST /api/user/assign_roles` | user_api | 分配角色 |
| `POST /api/user/assign_depts` | user_api | 分配部门 |
| `POST /api/role/` | role_api | 创建角色 |
| `GET /api/role/{role_id}` | role_api | 获取角色 |
| `PUT /api/role/{role_id}` | role_api | 更新角色 |
| `DELETE /api/role/{role_id}` | role_api | 删除角色 |
| `POST /api/role/list` | role_api | 角色分页列表 |
| `POST /api/role/assign-menus` | role_api | 分配菜单权限 |
| `POST /api/menu/` | menu_api | 创建菜单 |
| `PUT /api/menu/{menu_id}` | menu_api | 更新菜单 |
| `DELETE /api/menu/{menu_id}` | menu_api | 删除菜单 |
| `POST /api/menu/list` | menu_api | 菜单列表 |
| `POST /api/menu/tree` | menu_api | 菜单树 |
| `POST /api/dept/` | dept_api | 创建部门 |
| `PUT /api/dept/{dept_id}` | dept_api | 更新部门 |
| `DELETE /api/dept/{dept_id}` | dept_api | 删除部门 |
| `POST /api/dept/list` | dept_api | 部门列表 |
| `POST /api/dept/tree` | dept_api | 部门树 |
| `POST /api/permission/` | permission_api | 权限相关 |
| `POST /api/config/` | config_api | 配置管理 |
| `POST /api/dict/` | dict_api | 字典管理 |
| `POST /api/log/` | log_api | 日志查询 |
| `POST /api/file/` | file_api | 文件管理 |

---

### 3.2 `admin_app` — 应用层

**路径**: `examples/tx_admin/admin_app/`  
**职责**: 编排领域服务、DTO 转换、事务协调

#### 3.2.1 目录结构

```
admin_app/src/
├── lib.rs               # 模块导出
├── mock/                # Mock 仓库实现
│   ├── mod.rs
│   ├── user_repo.rs     # MockUserRepository
│   ├── role_repo.rs
│   ├── menu_repo.rs
│   ├── department_repo.rs
│   ├── permission_repo.rs
│   ├── config_repo.rs
│   ├── dict_repo.rs     # MockDictTypeRepository + MockDictDataRepository
│   ├── log_repo.rs      # MockOperateLogRepository + MockLoginLogRepository
│   └── file_repo.rs     # MockFileRepository + MockFileConfigRepository
├── auth/                # 认证应用服务
│   ├── mod.rs
│   ├── app_service.rs   # AuthAppService
│   └── dto.rs           # LoginCommand / LoginResponse / UserInfoResponse
├── user/                # 用户应用服务
│   ├── mod.rs
│   ├── app_service.rs   # UserAppService
│   └── dto.rs           # CreateUserCommand / UpdateUserCommand / UserResponse
├── role/                # 角色应用服务
│   ├── mod.rs
│   ├── app_service.rs   # RoleAppService
│   └── dto.rs
├── menu/                # 菜单应用服务
│   ├── mod.rs
│   ├── app_service.rs   # MenuAppService
│   └── dto.rs
├── department/          # 部门应用服务
│   ├── mod.rs
│   ├── app_service.rs   # DepartmentAppService
│   └── dto.rs
├── permission/          # 权限应用服务
│   ├── mod.rs
│   ├── app_service.rs   # PermissionAppService
│   └── dto.rs
├── config/              # 配置应用服务
│   ├── mod.rs
│   ├── app_service.rs   # ConfigAppService
│   └── dto.rs
├── dictionary/          # 字典应用服务
│   ├── mod.rs
│   ├── app_service.rs   # DictTypeAppService + DictDataAppService
│   └── dto.rs
├── log/                 # 日志应用服务
│   ├── mod.rs
│   ├── app_service.rs   # OperateLogAppService + LoginLogAppService
│   └── dto.rs
└── file/                # 文件应用服务
    ├── mod.rs
    ├── app_service.rs   # FileAppService
    └── dto.rs
```

#### 3.2.2 AppService 职责模式

每个 `XxxAppService` 遵循统一模式：

```rust
pub struct XxxAppService {
    xxx_service: Arc<XxxService>,  // 注入领域服务
}

impl XxxAppService {
    pub async fn create_xxx(&self, cmd: CreateXxxCommand, creator: Option<String>) -> AppResult<XxxResponse> {
        // 1. 参数校验 / 唯一性检查
        // 2. 调用领域服务执行业务逻辑
        // 3. 转换为 DTO 响应
    }
}
```

#### 3.2.3 DTO 转换约定

- **Command**: 输入 DTO，定义在 `admin_app/src/xxx/dto.rs`
- **Response**: 输出 DTO，直接复用 `admin_proto::admin::xxx::XxxResponse`（通过 type alias）
- **转换函数**: `xxx_to_response()` 或 `XxxResponse::from()`

#### 3.2.4 Mock 仓库实现

所有 Mock 仓库使用 `RwLock<HashMap<u64, T>>` 作为内存存储：

```rust
pub struct MockUserRepository {
    users: RwLock<HashMap<u64, User>>,
    user_roles: RwLock<HashMap<u64, Vec<u64>>>,
    user_depts: RwLock<HashMap<u64, Vec<u64>>>,
}
```

**注意**: Mock 仓库在进程重启后数据丢失，仅用于开发/测试。

#### 3.2.5 测试

```
admin_app/tests/
├── common/mod.rs        # 测试辅助：创建各模块 Service + AppService 的工厂函数
├── auth_test.rs
├── user_test.rs
├── role_test.rs
├── menu_test.rs
├── department_test.rs
├── permission_test.rs
├── config_test.rs
├── dict_test.rs
├── file_test.rs
├── log_test.rs
├── integration_test.rs  # 单模块集成测试
└── workflow_test.rs     # 全流程集成测试（部门→角色→用户→菜单→配置→字典→文件→日志）
```

---

### 3.3 `admin_domain` — 领域层

**路径**: `examples/tx_admin/admin_domain/`  
**职责**: 领域模型定义、业务规则、仓库接口

#### 3.3.1 目录结构

```
admin_domain/src/
├── lib.rs               # 模块导出 + 重导出 AggregateRoot 宏
├── shared/              # 共享领域基础
│   ├── mod.rs
│   ├── model/
│   │   ├── mod.rs       # Entity / AggregateRoot trait + DomainEvent + AuditFields
│   │   └── value_object.rs  # DeletedStatus + TenantId
│   └── repository.rs    # RepositoryError 枚举
├── user/                # 用户子域
│   ├── mod.rs
│   ├── model/
│   │   ├── mod.rs
│   │   ├── aggregate.rs # User 聚合根
│   │   ├── value_object.rs  # UserQuery / LoginUser / UserStatus / Sex
│   │   ├── event.rs     # (复用 shared DomainEvent)
│   │   └── tests.rs
│   ├── repository/mod.rs  # UserRepository trait
│   └── service/mod.rs     # UserService 领域服务
├── role/                # 角色子域
│   ├── model/
│   │   ├── aggregate.rs # Role 聚合根
│   │   └── value_object.rs  # RoleQuery
│   ├── repository/mod.rs  # RoleRepository trait
│   └── service/mod.rs     # RoleService
├── menu/                # 菜单子域
│   ├── model/
│   │   ├── aggregate.rs # Menu 聚合根（支持树形结构）
│   │   └── value_object.rs  # MenuQuery / MenuTreeNode
│   ├── repository/mod.rs
│   └── service/mod.rs
├── department/          # 部门子域
│   ├── model/
│   │   ├── aggregate.rs # Department 聚合根（支持树形结构）
│   │   └── value_object.rs  # DeptQuery / DeptTreeNode
│   ├── repository/mod.rs
│   └── service/mod.rs
├── permission/          # 权限子域
│   ├── model/
│   │   └── value_object.rs  # PermissionType / PermissionCheck
│   ├── repository/mod.rs  # PermissionRepository trait
│   └── service/mod.rs
├── config/              # 配置子域
│   ├── model/
│   │   ├── aggregate.rs # Config 聚合根
│   │   └── value_object.rs
│   ├── repository/mod.rs
│   └── service/mod.rs
├── dictionary/          # 字典子域
│   ├── model/
│   │   ├── aggregate.rs # DictType + DictData 聚合根
│   │   └── value_object.rs
│   ├── repository/mod.rs
│   └── service/mod.rs
├── file/                # 文件子域
│   ├── model/
│   │   ├── aggregate.rs # File + FileConfig 聚合根
│   │   └── value_object.rs
│   ├── repository/mod.rs
│   └── service/mod.rs
└── log/                 # 日志子域
    ├── model/
    │   ├── aggregate.rs # OperateLog + LoginLog 聚合根
    │   └── value_object.rs
    ├── repository/mod.rs
    └── service/mod.rs
```

#### 3.3.2 领域模型基础 (shared)

**Entity trait**:
```rust
pub trait Entity {
    type Id: Copy + Eq + std::hash::Hash;
    fn id(&self) -> Self::Id;
}
```

**AggregateRoot trait**:
```rust
pub trait AggregateRoot: Entity {
    fn events(&self) -> &[DomainEvent];
    fn clear_events(&mut self);
    fn add_event(&mut self, event: DomainEvent);
}
```

**DomainEvent 枚举** — 统一的领域事件类型，覆盖所有子域的增删改操作。

**AuditFields** — 所有实体共享的审计字段：
```rust
pub struct AuditFields {
    pub creator: Option<String>,
    pub create_time: Timestamp,
    pub updater: Option<String>,
    pub update_time: Timestamp,
    pub deleted: DeletedStatus,
}
```

**DeletedStatus** — 软删除标记：`Normal = 0` / `Deleted = 1`

**TenantId** — 租户 ID 值对象（新类型包装 u64）

#### 3.3.3 聚合根汇总

| 聚合根 | ID 类型 | 关键业务方法 |
|--------|---------|-------------|
| `User` | u64 | `create()`, `set_basic_info()`, `change_status()`, `change_password()`, `record_login()`, `set_roles()`, `set_departments()`, `soft_delete()` |
| `Role` | u64 | `create()`, `update_info()`, `change_status()`, `set_menus()`, `soft_delete()` |
| `Menu` | u64 | `create()`, `update_info()`, `change_status()`, `soft_delete()`, `is_directory()`, `is_menu()`, `is_button()`, `is_root()` |
| `Department` | u64 | `create()`, `update_info()`, `change_status()`, `soft_delete()`, `is_root()` |
| `Config` | u64 | `create()`, `update_info()`, `soft_delete()` |
| `DictType` | u64 | `create()`, `update_info()`, `change_status()`, `soft_delete()` |
| `DictData` | u64 | `create()`, `update_info()`, `change_status()`, `soft_delete()` |
| `File` | u64 | `create()`, `soft_delete()` |
| `FileConfig` | i32 | `create()` |
| `OperateLog` | u64 | `create()`, `with_request()` |
| `LoginLog` | u64 | `create()` |

#### 3.3.4 Repository trait 汇总

| Trait | 关键方法 |
|-------|---------|
| `UserRepository` | `find_by_id`, `find_by_username`, `find_page`, `insert`, `update`, `soft_delete`, `exists_by_username/email/mobile`, `bind_roles`, `bind_departments`, `get_role_ids`, `get_dept_ids` |
| `RoleRepository` | `find_by_id`, `find_by_code`, `find_by_ids`, `find_page`, `insert`, `update`, `soft_delete`, `exists_by_code`, `bind_menus`, `get_menu_ids` |
| `MenuRepository` | `find_by_id`, `find_all`, `find_by_ids`, `find_by_parent_id`, `insert`, `update`, `soft_delete`, `has_children` |
| `DepartmentRepository` | `find_by_id`, `find_all`, `find_by_parent_id`, `insert`, `update`, `soft_delete`, `has_children` |
| `PermissionRepository` | `find_by_role_ids`, `find_by_user_id`, `find_all` |
| `ConfigRepository` | `find_by_id`, `find_by_key`, `find_page`, `insert`, `update`, `soft_delete` |
| `DictTypeRepository` | `find_by_id`, `find_by_type`, `find_all`, `insert`, `update`, `soft_delete` |
| `DictDataRepository` | `find_by_id`, `find_by_dict_type`, `insert`, `update`, `soft_delete` |
| `FileRepository` | `find_by_id`, `find_page`, `insert`, `soft_delete` |
| `FileConfigRepository` | `find_by_id` |
| `OperateLogRepository` | `find_by_id`, `find_page`, `insert` |
| `LoginLogRepository` | `find_by_id`, `find_page`, `insert` |

#### 3.3.5 领域服务汇总

| 服务 | 依赖的 Repository | 职责 |
|------|-------------------|------|
| `UserService` | UserRepository + PermissionRepository | 用户 CRUD、密码管理、角色/部门绑定、登录信息构建 |
| `RoleService` | RoleRepository | 角色 CRUD、菜单权限分配 |
| `MenuService` | MenuRepository | 菜单 CRUD、菜单树构建 |
| `DepartmentService` | DepartmentRepository | 部门 CRUD、部门树构建 |
| `PermissionService` | PermissionRepository | 权限查询、权限校验 |
| `ConfigService` | ConfigRepository | 配置 CRUD |
| `DictTypeService` | DictTypeRepository | 字典类型 CRUD |
| `DictDataService` | DictDataRepository | 字典数据 CRUD |
| `FileService` | FileRepository + FileConfigRepository | 文件上传记录管理 |
| `OperateLogService` | OperateLogRepository | 操作日志记录 |
| `LoginLogService` | LoginLogRepository | 登录日志记录 |

---

### 3.4 `admin_proto` — 传输层

**路径**: `examples/tx_admin/admin_proto/`  
**职责**: 定义 Protobuf 消息和服务，生成 Rust 代码

#### 3.4.1 Proto 文件

```
admin_proto/protos/
├── common.proto         # Empty / PageRequest / PageResponse
├── auth.proto           # LoginRequest / LoginResponse / GetUserInfoRequest / ...
├── user.proto           # CreateUserRequest / UpdateUserRequest / UserResponse / ...
├── role.proto           # CreateRoleRequest / RoleResponse / ...
├── menu.proto           # CreateMenuRequest / MenuResponse / ...
├── department.proto     # CreateDeptRequest / DeptResponse / ...
├── permission.proto     # PermissionCheckRequest / GetUserPermissionsRequest / ...
├── config.proto         # CreateConfigRequest / ConfigResponse / ...
├── dictionary.proto     # CreateDictTypeRequest / DictTypeResponse / ...
├── log.proto            # CreateOperateLogRequest / OperateLogResponse / ...
└── file.proto           # UploadFileRequest / FileResponse / ...
```

#### 3.4.2 代码生成配置

```rust
// build.rs 关键配置
tonic_build::configure()
    .out_dir("src/pb")                                    // 输出到 src/pb/
    .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")  // 所有 message 加 serde
    .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")             // camelCase
    .field_attribute("optional", "#[serde(skip_serializing_if = \"Option::is_none\")]")
    .field_attribute("uint64", "#[serde(with = \"crate::serde_u64\")]")      // u64 序列化为字符串
    .compile_protos(...)
```

**注意**: `u64` 在 JSON 中序列化为字符串，避免 JavaScript 精度丢失。

---

### 3.5 `admin_macros` — 派生宏

**路径**: `examples/tx_admin/admin_macros/`  
**职责**: 提供 `#[derive(AggregateRoot)]` 宏

**要求**: 结构体必须包含 `id` 字段和 `events: Vec<DomainEvent>` 字段。

**自动生成**: `Entity` 和 `AggregateRoot` trait 实现。

---

### 3.6 `admin_infra` — 基础设施层（预留）

**路径**: `examples/tx_admin/admin_infra/`  
**当前状态**: 空壳，仅包含占位代码。  
**未来用途**: 存放真实数据库仓库实现（如 SQLite、PostgreSQL、MySQL）。

---

## 4. 依赖注入框架 (`tx-di-core`)

### 4.1 核心概念

| 概念 | 说明 |
|------|------|
| **组件 (Component)** | 通过 `#[tx_comp]` 标记的结构体，自动注册到 DI 容器 |
| **作用域 (Scope)** | `Singleton`（全局单例）/ `Prototype`（每次注入新建） |
| **BuildContext** | 构建上下文，负责注册、拓扑排序、构建 App |
| **App** | 运行时容器，存储所有已初始化的组件实例 |

### 4.2 组件生命周期

```
1. #[tx_comp] 注册 → COMPONENT_REGISTRY (linkme 分布式切片)
2. BuildContext::new() → 拓扑排序 → 按序注册工厂
3. BuildContext::build() → 创建 App
4. App::ins_run() →
   a. init()        — 同步初始化（按 init_sort 排序）
   b. async_init()  — 异步初始化（按 init_sort 排序）
   c. comp_run()    — 并行运行所有 async_run 组件
5. App::waiting_exit() → 等待 Ctrl+C / SIGTERM → graceful shutdown
```

### 4.3 关键 API

```rust
// 创建容器
let ctx = BuildContext::new(Some("config.toml"));

// 注册工厂
ctx.register_factory_boxed(type_id, scope, factory_fn);

// 注入组件
let component: Arc<MyComponent> = app.inject::<MyComponent>();

// 尝试注入（不 panic）
let component: Option<Arc<MyComponent>> = app.try_inject::<MyComponent>();
```

### 4.4 本项目中的 DI 使用

本项目的 DI 使用方式比较特殊：**admin_api 中的 `AdminPlugin` 是通过 DI 注册的组件，但实际的业务服务组装是在 `services.rs` 中手动完成的**（通过 `OnceLock` 全局静态变量）。这意味着：

- ✅ 框架的 HTTP 服务器、日志等基础设施通过 DI 管理
- ⚠️ 业务服务层目前是手动组装，未完全利用 DI 框架的注入能力

---

## 5. 公共库详解

### 5.1 `tx_common` — 通用工具

| 模块 | 功能 |
|------|------|
| `api_r` | `ApiR<T>` / `ApiRes` — 统一 API 响应结构 |
| `page` | `Page<T>` — 分页结果封装 |
| `id` | 雪花算法 ID 生成器（无锁 CAS、闰秒处理、批量生成） |
| `date` | `FormattedDateTime` — 格式化日期时间 |

### 5.2 `tx_error` — 统一错误

**三种错误形态**:
- `AppError::ErrCode` — 业务错误码（零堆分配）
- `AppError::WithContext` — 带动态上下文
- `AppError::Internal` — 框架/IO/第三方库错误（anyhow 包装）

**错误码定义** (通过 `#[derive(CodeMsg)]` 宏):
```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("REPOSITORY")]
pub enum RepositoryError {
    #[err(1000, "记录不存在")] Database,
    #[err(1001, "Not found")]  NotFound,
    #[err(1002, "Duplicate entry")] Duplicate,
    #[err(1003, "Validation error")] Validation,
    #[err("Internal error")] Internal,
}
```

### 5.3 `tx_di_axum` — Web 插件

提供 axum 集成：HTTP 服务器启动、路由注册、中间件层（API 日志等）。

### 5.4 `tx_di_log` — 日志插件

提供 tracing 日志初始化和配置。

---

## 6. 数据流详解

### 6.1 典型请求流程（以"创建用户"为例）

```
HTTP POST /api/user/
  │
  ▼
user_api::create_user()          ← 接口层
  │ 1. 反序列化 JSON → CreateUserRequest (proto)
  │ 2. 转换为 CreateUserCommand (app DTO)
  │ 3. 调用 services::get().user.create_user(cmd, None)
  │
  ▼
UserAppService::create_user()    ← 应用层
  │ 1. 检查邮箱唯一性
  │ 2. 检查手机号唯一性
  │ 3. 调用 UserService::create_user()
  │ 4. 设置可选字段并更新
  │ 5. 分配角色 / 部门
  │ 6. 转换为 UserResponse
  │
  ▼
UserService::create_user()       ← 领域层
  │ 1. 检查用户名唯一性
  │ 2. 生成雪花 ID
  │ 3. User::create() → 创建聚合根 + 添加领域事件
  │ 4. user_repo.insert()
  │
  ▼
MockUserRepository::insert()     ← 基础设施层
  │ HashMap::insert()（内存存储）
  │
  ▼
返回 User → UserResponse → ApiR::success() → JSON
```

### 6.2 认证流程

```
POST /api/auth/login
  │
  ▼
AuthAppService::login()
  │ 1. UserService::get_by_username() → 查找用户
  │ 2. 检查用户状态（active / locked）
  │ 3. 验证密码（当前明文比较）
  │ 4. UserService::build_login_user() → 构建登录信息
  │    - 获取角色 ID 列表
  │    - 获取部门 ID 列表
  │    - 获取权限列表
  │ 5. UserService::record_login() → 记录登录 IP 和时间
  │
  ▼
返回 LoginResponse (user_id, username, permissions, ...)
```

---

## 7. 已知问题与待办事项

### 7.1 架构层面

| # | 问题 | 严重度 | 说明 |
|---|------|--------|------|
| A1 | 仓库层仅有 Mock 实现 | 🔴 高 | `admin_infra` 为空壳，无真实数据库实现，数据重启丢失 |
| A2 | 密码明文存储和比较 | 🔴 高 | `AuthAppService::login()` 中 `user.password != cmd.password` 是明文比较 |
| A3 | 服务组装未利用 DI | 🟡 中 | `services.rs` 使用 `OnceLock` 手动组装，未通过 DI 框架的 `inject()` 注入 |
| A4 | Token 未实现 | 🟡 中 | `auth_api.rs` 中 `token: String::new()`，登录响应 token 为空 |
| A5 | gRPC 服务已注释 | 🟢 低 | `main.rs` 中 gRPC 启动代码被注释，gRPC 服务未实际运行 |
| A6 | 领域事件未发布 | 🟢 低 | `DomainEvent` 被收集但从未实际发布或处理 |

### 7.2 代码层面

| # | 问题 | 位置 | 说明 |
|---|------|------|------|
| C1 | 硬编码配置路径 | `main.rs:39` | `r"D:\proj\tx_di\examples\tx_admin\config\config.toml"` 硬编码 |
| C2 | DictType/DictData 手动实现 AggregateRoot | `dictionary/model/aggregate.rs` | 未使用 `#[derive(AggregateRoot)]` 宏 |
| C3 | Config 手动实现 AggregateRoot | `config/model/aggregate.rs` | 同上 |
| C4 | File 手动实现 AggregateRoot | `file/model/aggregate.rs` | 同上 |
| C5 | Log 手动实现 AggregateRoot | `log/model/aggregate.rs` | 同上 |
| C6 | get_user_info 重复查询 | `auth/app_service.rs:72-73` | `user_service.get_user()` 被调用了两次 |
| C7 | 部分 API 返回值不一致 | `role_api.rs` | `RoleResponse` 需要手动映射，未直接使用 proto 类型 |

---

## 8. Bug 修复指引

### 8.1 如何定位 Bug

1. **确定出错层**: 根据错误响应的 `code` 和 `msg` 定位是接口层、应用层还是领域层
2. **查看错误码**: `RepositoryError` 的 code 在 `1000-1003` 范围
3. **追踪请求链路**: HTTP Handler → AppService → DomainService → Repository

### 8.2 常见 Bug 模式

**模式 1: 数据查询返回空**
- 检查 Mock 仓库的 HashMap 中是否有数据
- 检查软删除过滤条件（`DeletedStatus::Normal`）
- 检查查询条件匹配逻辑

**模式 2: 唯一性校验失败**
- 检查 `exists_by_xxx()` 方法的比较逻辑
- Mock 仓库中是否已有同名记录

**模式 3: 关联数据丢失**
- 检查 `user_roles` / `user_depts` / `role_menus` 等关联表
- 检查 `bind_xxx()` / `get_xxx_ids()` 方法

### 8.3 添加新的错误码

```rust
// 1. 在对应模块定义错误枚举
#[derive(Debug, Copy, Clone, PartialEq, Eq, CodeMsg)]
#[err("MY_MODULE")]
pub enum MyModuleError {
    #[err(2001, "自定义错误信息")]
    CustomError,
}

// 2. 在业务代码中使用
return Err(MyModuleError::CustomError)?;
```

---

## 9. 功能开发指引

### 9.1 新增业务模块（完整 Checklist）

假设要新增"通知 (Notification)"模块：

#### Step 1: Proto 定义
```
admin_proto/protos/notification.proto
```
定义请求/响应消息，然后运行 `cargo build -p admin_proto` 生成代码。

#### Step 2: 领域层
```
admin_domain/src/notification/
├── mod.rs
├── model/
│   ├── mod.rs
│   ├── aggregate.rs     # Notification 聚合根（带 #[derive(AggregateRoot)]）
│   ├── value_object.rs  # NotificationQuery 等值对象
│   ├── event.rs         # (复用 shared DomainEvent，需在其中添加新事件)
│   └── tests.rs
├── repository/mod.rs    # NotificationRepository trait
└── service/mod.rs       # NotificationService 领域服务
```

**关键步骤**:
- 在 `shared/model/mod.rs` 的 `DomainEvent` 枚举中添加新事件变体
- 聚合根使用 `#[derive(AggregateRoot)]` 宏
- Repository trait 使用 `#[async_trait]`

#### Step 3: 应用层
```
admin_app/src/notification/
├── mod.rs
├── app_service.rs   # NotificationAppService
└── dto.rs           # Command / Response DTO + 转换函数
```

#### Step 4: Mock 仓库
```
admin_app/src/mock/notification_repo.rs
```
实现 `NotificationRepository` trait，使用 `RwLock<HashMap<u64, Notification>>`。

#### Step 5: 注册到服务表
```rust
// admin_api/src/services.rs
// 1. 添加字段
pub struct Svc {
    // ...
    pub notification: NotificationAppService,
}

// 2. 在 init_services() 中组装
let notification_repo = Arc::new(MockNotificationRepository::new());
let notification_svc = Arc::new(NotificationService::new(notification_repo.clone()));
let notification = NotificationAppService::new(notification_svc.clone());
```

#### Step 6: 接口层
```
admin_api/src/interfaces/api/notification_api.rs   # HTTP handler
admin_api/src/interfaces/grpc/notification_service.rs  # gRPC handler（可选）
```

在 `api/mod.rs` 的 `router()` 中注册路由：
```rust
.nest("/api/notification", notification_api::router(app.clone()))
```

#### Step 7: 测试
```
admin_app/tests/notification_test.rs
```
参考 `tests/common/mod.rs` 添加辅助工厂函数。

### 9.2 修改现有 API

1. **修改 Proto**: 更新 `admin_proto/protos/xxx.proto`，重新生成
2. **修改 DTO**: 更新 `admin_app/src/xxx/dto.rs`
3. **修改业务逻辑**: 更新 `admin_app/src/xxx/app_service.rs` 或 `admin_domain/src/xxx/service/mod.rs`
4. **修改接口**: 更新 `admin_api/src/interfaces/api/xxx_api.rs`
5. **运行测试**: `cargo test -p admin_app`

### 9.3 接入真实数据库

1. 在 `admin_infra` 中实现各 Repository trait
2. 添加数据库依赖（如 `sqlx` / `sea-orm` / `rusqlite`）
3. 修改 `services.rs`，将 `MockXxxRepository` 替换为真实实现
4. 或者通过 DI 框架注入（更推荐）

---

## 10. 构建与运行

### 10.1 前置条件

- Rust 1.75+
- protoc（通过 `protoc_bin_vendored` crate 自动提供，无需手动安装）

### 10.2 构建命令

```bash
# 构建整个项目
cargo build

# 仅构建 admin_api
cargo build -p admin_api

# 运行
cargo run -p admin_api

# 运行测试
cargo test -p admin_app

# 运行特定测试
cargo test -p admin_app --test workflow_test
```

### 10.3 配置文件

配置文件路径在 `main.rs` 中硬编码：
```rust
let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
```

配置内容由 `AppAllConfig` 解析，格式为 TOML。

---

## 11. 架构决策记录 (ADR)

### ADR-001: 采用 Mock 内存仓库而非真实数据库

**状态**: Accepted（当前阶段）  
**背景**: 项目初期需要快速验证 DDD 分层架构和 DI 框架的集成效果。  
**决策**: 使用 `RwLock<HashMap<u64, T>>` 实现 Mock 仓库。  
**后果**:
- ✅ 开发速度快，无需数据库环境
- ✅ 测试简单，无外部依赖
- ❌ 数据重启丢失
- ❌ 无法验证并发事务、SQL 性能等

### ADR-002: Proto 生成的类型作为传输 DTO

**状态**: Accepted  
**背景**: 同时支持 HTTP JSON 和 gRPC，需要统一的传输对象。  
**决策**: 使用 `tonic-build` 生成 proto DTO，HTTP 和 gRPC 共用。  
**后果**:
- ✅ 一套定义，两种协议
- ✅ 自动生成 serde 序列化
- ❌ proto 类型与领域模型之间需要手动转换
- ❌ u64 需要特殊处理（JSON 字符串序列化）

### ADR-003: 全局 OnceLock 服务注册表

**状态**: Accepted（临时方案）  
**背景**: 需要在 axum handler 中访问应用服务。  
**决策**: 使用 `OnceLock<Arc<Svc>>` 作为全局服务注册表。  
**后果**:
- ✅ 实现简单，handler 中直接 `services::get()`
- ❌ 绕过了 DI 框架的注入机制
- ❌ 不利于测试时替换服务实现

---

## 12. 快速参考

### 12.1 关键文件速查

| 需求 | 文件路径 |
|------|---------|
| 启动入口 | `admin_api/src/main.rs` |
| 路由注册 | `admin_api/src/interfaces/api/mod.rs` |
| 服务组装 | `admin_api/src/services.rs` |
| 统一响应 | `tx_common/src/api_r.rs` |
| 分页封装 | `tx_common/src/page.rs` |
| ID 生成 | `tx_common/src/id.rs` |
| 错误定义 | `tx_error/src/error.rs` |
| 领域事件 | `admin_domain/src/shared/model/mod.rs` |
| 审计字段 | `admin_domain/src/shared/model/mod.rs` (AuditFields) |
| 软删除 | `admin_domain/src/shared/model/value_object.rs` (DeletedStatus) |

### 12.2 命名约定

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
