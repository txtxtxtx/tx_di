# 通用后台管理系统 - 完整DDD设计文档（Rust + Axum + gRPC + sa-token）

> **版本**: v3.1-Rust
> **状态**: 生产可用
> **认证框架**: sa-token-rust
> **最后更新**: 2026-06-09
```rust
trait A {
    fn a(&self)->String;
}
struct B {
    name:String
}
impl A for B {
    fn a(&self)->String {
        "B".to_string()
    }
}
struct C{
    a:Arc<dyn A>
}
// 我的想法是编译期直接替换掉 Arc<dyn A> 为 Arc<B>

fn test(){
    let a: Arc<dyn A> = Arc::new(B{name:"a".to_string()});
}
```
---

## 目录

1. [架构总览](#一架构总览)
2. [分层架构设计](#二分层架构设计)
3. [限界上下文与领域模型](#三限界上下文与领域模型)
4. [认证与授权设计（sa-token）](#四认证与授权设计sa-token)
5. [数据权限设计](#五数据权限设计)
6. [缓存策略设计](#六缓存策略设计)
7. [gRPC API设计规范](#七grpc-api设计规范)
8. [安全设计](#八安全设计)
9. [性能优化方案](#九性能优化方案)
10. [监控与告警设计](#十监控与告警设计)
11. [部署架构](#十一部署架构)
12. [扩展性设计](#十二扩展性设计)
13. [数据库设计](#十三数据库设计)
14. [附录](#十四附录)

---

## 一、架构总览

### 1.1 系统定位

通用后台管理系统是企业级应用的基础设施层，提供：
- 统一的身份认证与访问控制（sa-token）
- 灵活的组织架构管理
- 完整的系统配置能力
- 全面的审计追踪

### 1.2 架构原则

| 原则 | 说明 |
|------|------|
| **领域驱动** | 以业务领域为核心，技术服务于业务 |
| **限界上下文隔离** | 明确的领域边界，降低耦合 |
| **CQRS模式** | 读写分离，优化查询性能 |
| **事件驱动** | 跨上下文通过领域事件通信 |
| **防御式设计** | 输入校验、异常处理、安全防护 |
| **可观测性** | 日志、指标、链路追踪 |
| **零成本抽象** | Rust的零成本抽象，编译时保证安全 |

### 1.3 技术栈选型

```
┌─────────────────────────────────────────────────────────────┐
│                        前端层                                │
│  Vue 3 + TypeScript + Element Plus + Pinia                  │
│  gRPC-Web / Connect-Web (gRPC浏览器支持)                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      反向代理层                              │
│  Nginx / Envoy (gRPC-Web 转 gRPC)                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    应用服务层 (Rust)                         │
│  Axum + Tonic (gRPC) + sa-token (认证授权)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    领域层 (DDD)                              │
│  聚合根、实体、值对象、领域服务、领域事件                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    基础设施层                                │
│  SQLx/SeaORM + Redis + MinIO/OSS + RabbitMQ/NATS            │
└─────────────────────────────────────────────────────────────┘
```

### 1.4 Rust 生态选型

| 组件 | 选型 | 说明 |
|------|------|------|
| **Web框架** | Axum | Tokio团队出品，Tower生态 |
| **gRPC框架** | Tonic | 基于Tokio的高性能gRPC实现 |
| **认证授权** | sa-token-rust | 轻量级、功能完整的认证框架 |
| **异步运行时** | Tokio | Rust事实标准异步运行时 |
| **数据库** | SQLx | 异步、编译时检查SQL |
| **ORM (可选)** | SeaORM | 功能丰富的异步ORM |
| **Redis** | redis-rs (异步) | Redis官方Rust客户端 |
| **序列化** | Serde + Prost | JSON + Protobuf |
| **日志** | tracing + tracing-subscriber | 结构化日志 |
| **错误处理** | anyhow + thiserror | 错误处理最佳实践 |
| **配置** | config-rs | 多格式配置支持 |
| **UUID** | uuid | UUID生成和解析（仅用于非ID场景） |
| **时间** | chrono | 时间日期处理 |
| **验证** | validator | 参数验证 |

### 1.5 sa-token-rust 核心特性

| 特性 | 说明 |
|------|------|
| **登录认证** | 支持多种登录方式，Token管理 |
| **权限认证** | 基于RBAC，支持通配符匹配 |
| **角色认证** | 角色检查，支持AND/OR逻辑 |
| **Session会话** | 分布式Session支持 |
| **JWT集成** | 内置JWT实现，支持多种算法 |
| **多框架支持** | 支持Axum、Actix-web等9种框架 |
| **多种存储** | 内存、Redis、数据库存储 |
| **事件监听** | 登录、登出、踢出下线事件 |
| **在线用户管理** | 实时在线状态跟踪 |
| **分布式支持** | 跨服务Session共享、SSO |

### 1.6 ID规范

> **重要**：系统所有ID统一使用 `u64` 类型，不使用UUID。

| ID类型 | 类型 | 说明 |
|--------|------|------|
| 用户ID | u64 | 雪花算法生成 |
| 角色ID | u64 | 雪花算法生成 |
| 权限ID | u64 | 雪花算法生成 |
| 菜单ID | u64 | 雪花算法生成 |
| 部门ID | u64 | 雪花算法生成 |
| 文件ID | u64 | 雪花算法生成 |
| 配置ID | u64 | 雪花算法生成 |
| 字典ID | u64 | 雪花算法生成 |
| 日志ID | u64 | 雪花算法生成 |

---

## 二、分层架构设计

### 2.1 分层模型

```
┌─────────────────────────────────────────────────────────────┐
│                    接口层 (Interface Layer)                  │
│  gRPC Service, Axum Handler, DTO, 参数校验                  │
├─────────────────────────────────────────────────────────────┤
│                    应用层 (Application Layer)                │
│  AppService, Command, Query, 事务管理, 事件发布              │
├─────────────────────────────────────────────────────────────┤
│                    领域层 (Domain Layer)                     │
│  Aggregate, Entity, ValueObject, DomainService, DomainEvent │
├─────────────────────────────────────────────────────────────┤
│                    基础设施层 (Infrastructure Layer)         │
│  Repository Impl, SQLx, Redis, MinIO, MessageQueue          │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 项目结构设计

```
admin-server/
├── Cargo.toml                      # 工作空间配置
├── proto/                          # Protobuf定义
│   ├── admin/
│   │   ├── user.proto
│   │   ├── role.proto
│   │   ├── permission.proto
│   │   ├── menu.proto
│   │   ├── department.proto
│   │   ├── file.proto
│   │   ├── config.proto
│   │   ├── dictionary.proto
│   │   ├── log.proto
│   │   └── auth.proto
│   └── common/
│       └── common.proto
│
├── crates/                         # 工作空间成员
│   ├── admin-proto/                # Protobuf生成代码
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   │
│   ├── admin-common/               # 公共模块
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs            # 错误定义
│   │       ├── result.rs           # 统一结果
│   │       ├── config.rs           # 配置
│   │       ├── id.rs               # ID生成器（雪花算法）
│   │       └── auth/               # sa-token集成
│   │           ├── mod.rs
│   │           ├── middleware.rs    # Axum中间件
│   │           ├── extractor.rs    # 提取器
│   │           └── interceptor.rs  # gRPC拦截器
│   │
│   ├── admin-domain/               # 领域层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── user/
│   │       │   ├── mod.rs
│   │       │   ├── model/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── user.rs
│   │       │   │   ├── value_object.rs
│   │       │   │   └── event.rs
│   │       │   ├── service/
│   │       │   │   └── mod.rs
│   │       │   └── repository/
│   │       │       └── mod.rs
│   │       ├── role/
│   │       ├── permission/
│   │       ├── menu/
│   │       ├── department/
│   │       ├── file/
│   │       ├── config/
│   │       ├── dictionary/
│   │       ├── log/
│   │       └── shared/
│   │           ├── mod.rs
│   │           ├── aggregate.rs
│   │           ├── entity.rs
│   │           └── value_object.rs
│   │
│   ├── admin-application/          # 应用层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── user/
│   │       ├── role/
│   │       ├── permission/
│   │       ├── menu/
│   │       ├── department/
│   │       ├── file/
│   │       ├── config/
│   │       ├── dictionary/
│   │       └── log/
│   │
│   ├── admin-infrastructure/       # 基础设施层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── persistence/
│   │       ├── cache/
│   │       ├── storage/
│   │       └── mq/
│   │
│   └── admin-server/               # 服务器启动
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── grpc/
│           ├── http/
│           └── startup.rs
│
├── migrations/                     # 数据库迁移
├── config/                         # 配置文件
├── docker/                         # Docker配置
└── scripts/                        # 脚本
```

---

## 三、限界上下文与领域模型

### 3.1 用户管理模块

#### 领域模型

```rust
// crates/admin-domain/src/user/model/user.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::shared::aggregate::AggregateRoot;
use crate::user::model::value_object::*;
use crate::user::model::event::*;

/// 用户聚合根 - 所有ID使用u64
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    // ---- 核心属性 ----
    id: u64,                          // 用户ID（雪花算法）
    username: Username,
    password_hash: PasswordHash,
    status: UserStatus,
    login_attempts: LoginAttempts,

    // ---- 值对象 ----
    profile: Profile,
    last_password_change: Option<DateTime<Utc>>,

    // ---- 关联 ----
    roles: Vec<UserRole>,
    department: Option<UserDepartment>,

    // ---- 审计字段 ----
    created_at: DateTime<Utc>,
    created_by: Option<u64>,
    updated_at: DateTime<Utc>,
    updated_by: Option<u64>,
    last_login_at: Option<DateTime<Utc>>,
    last_login_ip: Option<String>,

    // ---- 领域事件 ----
    #[serde(skip)]
    events: Vec<UserEvent>,
}

impl AggregateRoot for User {
    type Id = u64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn events(&self) -> &[UserEvent] {
        &self.events
    }

    fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl User {
    /// 创建新用户
    pub fn create(
        id: u64,
        username: Username,
        password_hash: PasswordHash,
        profile: Profile,
        created_by: u64,
    ) -> Result<Self, UserError> {
        let now = Utc::now();
        let user = Self {
            id,
            username,
            password_hash,
            status: UserStatus::Active,
            login_attempts: LoginAttempts::default(),
            profile,
            last_password_change: Some(now),
            roles: Vec::new(),
            department: None,
            created_at: now,
            created_by: Some(created_by),
            updated_at: now,
            updated_by: None,
            last_login_at: None,
            last_login_ip: None,
            events: Vec::new(),
        };

        user.events.push(UserEvent::Created {
            user_id: user.id,
            username: user.username.clone(),
            created_at: now,
        });

        Ok(user)
    }

    /// 修改密码
    pub fn change_password(
        &mut self,
        old_password: &str,
        new_password: &str,
        policy: &PasswordPolicy,
    ) -> Result<(), UserError> {
        if !self.password_hash.verify(old_password) {
            return Err(UserError::IncorrectPassword);
        }

        policy.validate(new_password)?;

        self.password_hash = PasswordHash::new(new_password)?;
        self.last_password_change = Some(Utc::now());
        self.updated_at = Utc::now();

        self.events.push(UserEvent::PasswordChanged {
            user_id: self.id,
            changed_at: Utc::now(),
        });

        Ok(())
    }

    /// 启用用户
    pub fn enable(&mut self) {
        if self.status == UserStatus::Active {
            return;
        }
        self.status = UserStatus::Active;
        self.login_attempts = LoginAttempts::default();
        self.updated_at = Utc::now();

        self.events.push(UserEvent::Enabled {
            user_id: self.id,
        });
    }

    /// 禁用用户
    pub fn disable(&mut self) {
        if self.status == UserStatus::Disabled {
            return;
        }
        self.status = UserStatus::Disabled;
        self.updated_at = Utc::now();

        self.events.push(UserEvent::Disabled {
            user_id: self.id,
        });
    }

    /// 锁定用户
    pub fn lock(&mut self) {
        self.status = UserStatus::Locked;
        self.login_attempts = LoginAttempts::locked();
        self.updated_at = Utc::now();

        self.events.push(UserEvent::Locked {
            user_id: self.id,
        });
    }

    /// 解锁用户
    pub fn unlock(&mut self) {
        self.status = UserStatus::Active;
        self.login_attempts = LoginAttempts::default();
        self.updated_at = Utc::now();

        self.events.push(UserEvent::Unlocked {
            user_id: self.id,
        });
    }

    /// 记录登录尝试
    pub fn record_login_attempt(&mut self, success: bool, max_attempts: u32) -> bool {
        if success {
            self.login_attempts = LoginAttempts::default();
            self.last_login_at = Some(Utc::now());
            return false;
        }

        self.login_attempts = self.login_attempts.increment();

        if self.login_attempts.is_exceeded(max_attempts) {
            self.lock();
            return true;
        }
        false
    }

    /// 分配角色
    pub fn assign_roles(&mut self, role_ids: Vec<u64>) {
        self.roles = role_ids
            .into_iter()
            .map(|role_id| UserRole::new(self.id, role_id))
            .collect();
        self.updated_at = Utc::now();
    }

    /// 分配部门
    pub fn assign_department(&mut self, department_id: u64) {
        self.department = Some(UserDepartment::new(self.id, department_id));
        self.updated_at = Utc::now();
    }

    /// 检查密码是否过期
    pub fn is_password_expired(&self, max_age: chrono::Duration) -> bool {
        match self.last_password_change {
            Some(last_change) => {
                let now = Utc::now();
                now - last_change > max_age
            }
            None => true,
        }
    }

    /// 验证用户是否可用于登录
    pub fn validate_for_login(&self) -> Result<(), UserError> {
        match self.status {
            UserStatus::Disabled => Err(UserError::UserDisabled),
            UserStatus::Locked => Err(UserError::UserLocked),
            UserStatus::Active => Ok(()),
        }
    }

    // Getter方法
    pub fn id(&self) -> u64 { self.id }
    pub fn username(&self) -> &Username { &self.username }
    pub fn status(&self) -> UserStatus { self.status }
    pub fn profile(&self) -> &Profile { &self.profile }
    pub fn roles(&self) -> &[UserRole] { &self.roles }
    pub fn department(&self) -> Option<&UserDepartment> { self.department.as_ref() }
    pub fn password_hash(&self) -> &PasswordHash { &self.password_hash }
    pub fn last_login_at(&self) -> Option<DateTime<Utc>> { self.last_login_at }
    pub fn last_login_ip(&self) -> Option<&String> { self.last_login_ip.as_ref() }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}
```

```rust
// crates/admin-domain/src/user/model/value_object.rs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// 用户名值对象
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    pub fn new(value: String) -> Result<Self, UserError> {
        let regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]{3,19}$").unwrap();
        if !regex.is_match(&value) {
            return Err(UserError::InvalidUsername);
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 密码哈希值对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHash {
    hash: String,
    algorithm: String,
}

impl PasswordHash {
    pub fn new(raw_password: &str) -> Result<Self, UserError> {
        use argon2::{
            password_hash::{
                rand_core::OsRng, PasswordHasher, SaltString,
            },
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(raw_password.as_bytes(), &salt)
            .map_err(|_| UserError::PasswordHashError)?
            .to_string();

        Ok(Self {
            hash,
            algorithm: "argon2".to_string(),
        })
    }

    pub fn verify(&self, raw_password: &str) -> bool {
        use argon2::{
            password_hash::{PasswordVerifier, PasswordHash as Argon2Hash},
            Argon2,
        };

        let parsed_hash = Argon2Hash::new(&self.hash);
        match parsed_hash {
            Ok(hash) => Argon2::default()
                .verify_password(raw_password.as_bytes(), &hash)
                .is_ok(),
            Err(_) => false,
        }
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

/// 个人信息值对象
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Profile {
    pub real_name: Option<String>,
    pub nickname: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(regex = "PHONE_REGEX")]
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub gender: Gender,
    pub birthday: Option<chrono::NaiveDate>,
    pub remark: Option<String>,
}

lazy_static::lazy_static! {
    static ref PHONE_REGEX: regex::Regex = regex::Regex::new(r"^1[3-9]\d{9}$").unwrap();
}

/// 性别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Unknown = 0,
    Male = 1,
    Female = 2,
}

/// 用户状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Active = 1,
    Disabled = 0,
    Locked = 2,
}

/// 登录尝试值对象
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoginAttempts {
    count: u32,
    last_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    locked: bool,
}

impl LoginAttempts {
    pub fn increment(&self) -> Self {
        Self {
            count: self.count + 1,
            last_attempt_at: Some(chrono::Utc::now()),
            locked: false,
        }
    }

    pub fn is_exceeded(&self, max_attempts: u32) -> bool {
        self.count >= max_attempts
    }

    pub fn locked() -> Self {
        Self {
            count: 0,
            last_attempt_at: Some(chrono::Utc::now()),
            locked: true,
        }
    }
}

/// 用户角色关联 - ID使用u64
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    user_id: u64,
    role_id: u64,
    assigned_at: chrono::DateTime<chrono::Utc>,
}

impl UserRole {
    pub fn new(user_id: u64, role_id: u64) -> Self {
        Self {
            user_id,
            role_id,
            assigned_at: chrono::Utc::now(),
        }
    }

    pub fn user_id(&self) -> u64 { self.user_id }
    pub fn role_id(&self) -> u64 { self.role_id }
}

/// 用户部门关联 - ID使用u64
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDepartment {
    user_id: u64,
    department_id: u64,
    is_primary: bool,
}

impl UserDepartment {
    pub fn new(user_id: u64, department_id: u64) -> Self {
        Self {
            user_id,
            department_id,
            is_primary: true,
        }
    }

    pub fn user_id(&self) -> u64 { self.user_id }
    pub fn department_id(&self) -> u64 { self.department_id }
}

/// 密码策略
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
    pub max_login_attempts: u32,
    pub password_max_age_days: u32,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 32,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: true,
            max_login_attempts: 5,
            password_max_age_days: 90,
        }
    }
}

impl PasswordPolicy {
    pub fn validate(&self, password: &str) -> Result<(), UserError> {
        if password.len() < self.min_length {
            return Err(UserError::PasswordTooShort);
        }
        if password.len() > self.max_length {
            return Err(UserError::PasswordTooLong);
        }
        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(UserError::PasswordNoUppercase);
        }
        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(UserError::PasswordNoLowercase);
        }
        if self.require_digit && !password.chars().any(|c| c.is_numeric()) {
            return Err(UserError::PasswordNoDigit);
        }
        if self.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(UserError::PasswordNoSpecialChar);
        }
        Ok(())
    }

    pub fn generate_initial_password(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let lower = "abcdefghijklmnopqrstuvwxyz";
        let digits = "0123456789";
        let special = "!@#$%^&*";
        let all = format!("{}{}{}{}", upper, lower, digits, special);

        let mut password = String::new();
        password.push(upper.chars().nth(rng.gen_range(0..upper.len())).unwrap());
        password.push(lower.chars().nth(rng.gen_range(0..lower.len())).unwrap());
        password.push(digits.chars().nth(rng.gen_range(0..digits.len())).unwrap());
        password.push(special.chars().nth(rng.gen_range(0..special.len())).unwrap());

        for _ in 4..self.min_length {
            password.push(all.chars().nth(rng.gen_range(0..all.len())).unwrap());
        }

        let mut chars: Vec<char> = password.chars().collect();
        use rand::seq::SliceRandom;
        chars.shuffle(&mut rng);

        chars.into_iter().collect()
    }
}
```

```rust
// crates/admin-domain/src/user/model/event.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 用户领域事件 - ID使用u64
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserEvent {
    Created {
        user_id: u64,
        username: String,
        created_at: DateTime<Utc>,
    },
    PasswordChanged {
        user_id: u64,
        changed_at: DateTime<Utc>,
    },
    Enabled {
        user_id: u64,
    },
    Disabled {
        user_id: u64,
    },
    Locked {
        user_id: u64,
    },
    Unlocked {
        user_id: u64,
    },
    LoggedIn {
        user_id: u64,
        login_ip: String,
        login_at: DateTime<Utc>,
    },
}
```

```rust
// crates/admin-domain/src/user/repository/mod.rs

use async_trait::async_trait;
use crate::user::model::user::User;

/// 用户仓储接口 - ID使用u64
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, RepositoryError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError>;
    async fn find_by_phone(&self, phone: &str) -> Result<Option<User>, RepositoryError>;
    async fn exists_by_username(&self, username: &str) -> Result<bool, RepositoryError>;
    async fn exists_by_email(&self, email: &str) -> Result<bool, RepositoryError>;
    async fn exists_by_phone(&self, phone: &str) -> Result<bool, RepositoryError>;
    async fn find_by_condition(
        &self,
        condition: UserQueryCondition,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<User>, i64), RepositoryError>;
    async fn find_by_role_id(&self, role_id: u64) -> Result<Vec<User>, RepositoryError>;
    async fn find_by_department_id(&self, dept_id: u64) -> Result<Vec<User>, RepositoryError>;
    async fn delete(&self, id: u64) -> Result<(), RepositoryError>;
}

/// 用户查询条件
#[derive(Debug, Clone, Default)]
pub struct UserQueryCondition {
    pub username: Option<String>,
    pub real_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: Option<i32>,
    pub department_id: Option<u64>,
    pub department_ids: Option<Vec<u64>>,
    pub role_id: Option<u64>,
    pub created_start: Option<chrono::DateTime<chrono::Utc>>,
    pub created_end: Option<chrono::DateTime<chrono::Utc>>,
}

/// 仓储错误
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found")]
    NotFound,

    #[error("Duplicate entry: {0}")]
    Duplicate(String),
}
```

```rust
// crates/admin-domain/src/user/service/mod.rs

use async_trait::async_trait;
use crate::user::model::user::User;
use crate::user::model::value_object::*;
use crate::user::repository::UserRepository;

/// 用户领域服务
pub struct UserService<R: UserRepository> {
    repository: R,
    password_policy: PasswordPolicy,
    id_generator: crate::shared::id::IdGenerator,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(
        repository: R,
        password_policy: PasswordPolicy,
        id_generator: crate::shared::id::IdGenerator,
    ) -> Self {
        Self {
            repository,
            password_policy,
            id_generator,
        }
    }

    /// 创建用户
    pub async fn create_user(
        &self,
        username: String,
        profile: Profile,
        role_ids: Vec<u64>,
        department_id: Option<u64>,
        created_by: u64,
    ) -> Result<User, UserServiceError> {
        // 验证用户名唯一
        if self.repository.exists_by_username(&username).await? {
            return Err(UserServiceError::UsernameAlreadyExists);
        }

        // 验证邮箱唯一
        if let Some(email) = &profile.email {
            if self.repository.exists_by_email(email).await? {
                return Err(UserServiceError::EmailAlreadyExists);
            }
        }

        // 验证手机号唯一
        if let Some(phone) = &profile.phone {
            if self.repository.exists_by_phone(phone).await? {
                return Err(UserServiceError::PhoneAlreadyExists);
            }
        }

        // 生成初始密码
        let initial_password = self.password_policy.generate_initial_password();

        // 生成用户ID
        let user_id = self.id_generator.next_id();

        // 创建用户聚合
        let username = Username::new(username)?;
        let password_hash = PasswordHash::new(&initial_password)?;
        let mut user = User::create(user_id, username, password_hash, profile, created_by)?;

        // 分配角色
        user.assign_roles(role_ids);

        // 分配部门
        if let Some(dept_id) = department_id {
            user.assign_department(dept_id);
        }

        // 保存
        self.repository.save(&user).await?;

        Ok(user)
    }

    /// 验证登录
    pub async fn validate_login(
        &self,
        username: &str,
        password: &str,
        client_ip: &str,
    ) -> Result<User, UserServiceError> {
        // 查找用户
        let mut user = self.repository
            .find_by_username(username)
            .await?
            .ok_or(UserServiceError::UserNotFound)?;

        // 检查用户状态
        user.validate_for_login()?;

        // 验证密码
        let password_matches = user.password_hash().verify(password);

        // 记录登录尝试
        let locked = user.record_login_attempt(
            password_matches,
            self.password_policy.max_login_attempts,
        );

        if locked {
            self.repository.save(&user).await?;
            return Err(UserServiceError::UserLockedAfterAttempts);
        }

        if !password_matches {
            self.repository.save(&user).await?;
            return Err(UserServiceError::IncorrectPassword);
        }

        // 检查密码是否过期
        let max_age = chrono::Duration::days(self.password_policy.password_max_age_days as i64);
        if user.is_password_expired(max_age) {
            return Err(UserServiceError::PasswordExpired);
        }

        // 保存登录信息
        self.repository.save(&user).await?;

        Ok(user)
    }

    /// 重置密码
    pub async fn reset_password(&self, user_id: u64) -> Result<String, UserServiceError> {
        let mut user = self.repository
            .find_by_id(user_id)
            .await?
            .ok_or(UserServiceError::UserNotFound)?;

        let new_password = self.password_policy.generate_initial_password();
        let password_hash = PasswordHash::new(&new_password)?;

        // 这里需要在User上添加reset_password方法
        // user.reset_password(password_hash);

        self.repository.save(&user).await?;

        Ok(new_password)
    }
}

/// 用户服务错误
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    #[error("User not found")]
    UserNotFound,

    #[error("Username already exists")]
    UsernameAlreadyExists,

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Phone already exists")]
    PhoneAlreadyExists,

    #[error("Incorrect password")]
    IncorrectPassword,

    #[error("User is disabled")]
    UserDisabled,

    #[error("User is locked")]
    UserLocked,

    #[error("User locked after too many attempts")]
    UserLockedAfterAttempts,

    #[error("Password expired")]
    PasswordExpired,

    #[error("Invalid username")]
    InvalidUsername,

    #[error("Password too short")]
    PasswordTooShort,

    #[error("Password too long")]
    PasswordTooLong,

    #[error("Password no uppercase")]
    PasswordNoUppercase,

    #[error("Password no lowercase")]
    PasswordNoLowercase,

    #[error("Password no digit")]
    PasswordNoDigit,

    #[error("Password no special character")]
    PasswordNoSpecialChar,

    #[error("Repository error: {0}")]
    Repository(#[from] crate::user::repository::RepositoryError),
}
```

### 3.2 ID生成器

```rust
// crates/admin-common/src/id.rs

use std::sync::atomic::{AtomicU64, Ordering};

/// 雪花算法ID生成器
pub struct IdGenerator {
    worker_id: u64,
    datacenter_id: u64,
    sequence: AtomicU64,
    last_timestamp: AtomicU64,
}

impl IdGenerator {
    pub fn new(worker_id: u64, datacenter_id: u64) -> Self {
        Self {
            worker_id,
            datacenter_id,
            sequence: AtomicU64::new(0),
            last_timestamp: AtomicU64::new(0),
        }
    }

    pub fn next_id(&self) -> u64 {
        let mut timestamp = Self::current_timestamp();

        // 等待下一毫秒
        while timestamp <= self.last_timestamp.load(Ordering::Relaxed) {
            timestamp = Self::current_timestamp();
        }

        if timestamp == self.last_timestamp.load(Ordering::Relaxed) {
            let sequence = self.sequence.fetch_add(1, Ordering::SeqCst) & 0xFFF;
            if sequence == 0 {
                // 序列号用完，等待下一毫秒
                while timestamp <= self.last_timestamp.load(Ordering::Relaxed) {
                    timestamp = Self::current_timestamp();
                }
            }
        } else {
            self.sequence.store(0, Ordering::SeqCst);
        }

        self.last_timestamp.store(timestamp, Ordering::SeqCst);

        // 生成ID
        ((timestamp - 1288834974657) << 22)
            | (self.datacenter_id << 17)
            | (self.worker_id << 12)
            | self.sequence.load(Ordering::Relaxed)
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// 全局ID生成器
lazy_static::lazy_static! {
    static ref ID_GENERATOR: IdGenerator = IdGenerator::new(1, 1);
}

/// 生成下一个ID
pub fn next_id() -> u64 {
    ID_GENERATOR.next_id()
}
```

---

## 四、认证与授权设计（sa-token）

### 4.1 sa-token 集成配置

```toml
# Cargo.toml

[dependencies]
# sa-token for Axum
sa-token-plugin-axum = { version = "0.1.14", features = ["redis"] }

# 或者使用完整功能
# sa-token-plugin-axum = { version = "0.1.14", features = ["full"] }
```

### 4.2 sa-token 初始化

```rust
// crates/admin-common/src/auth/mod.rs

use sa_token_plugin_axum::prelude::*;
use std::sync::Arc;

/// sa-token 配置
pub struct SaTokenConfig {
    /// Token名称
    pub token_name: String,
    /// Token有效期（秒）
    pub timeout: u64,
    /// Token最低活跃时间（秒）
    pub active_timeout: u64,
    /// 是否允许同一账号多地同时登录
    pub is_concurrent: bool,
    /// 在多人登录同一账号时，是否共用一个Token
    pub is_share: bool,
    /// Token风格
    pub token_style: TokenStyle,
}

impl Default for SaTokenConfig {
    fn default() -> Self {
        Self {
            token_name: "Authorization".to_string(),
            timeout: 86400,           // 24小时
            active_timeout: 1800,     // 30分钟
            is_concurrent: true,
            is_share: false,
            token_style: TokenStyle::Uuid,
        }
    }
}

/// 初始化 sa-token 状态
pub async fn init_sa_token_state(
    redis_url: &str,
    config: SaTokenConfig,
) -> Result<SaTokenState, Box<dyn std::error::Error>> {
    // 创建Redis存储
    let storage = Arc::new(
        RedisStorage::new(redis_url).await?
    );

    // 构建SaTokenState
    let state = SaTokenState::builder()
        .storage(storage)
        .token_name(&config.token_name)
        .timeout(config.timeout)
        .active_timeout(config.active_timeout)
        .is_concurrent(config.is_concurrent)
        .is_share(config.is_share)
        .token_style(config.token_style)
        .build();

    Ok(state)
}
```

### 4.3 Axum 中间件集成

```rust
// crates/admin-common/src/auth/middleware.rs

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sa_token_plugin_axum::prelude::*;

/// 登录认证中间件
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // sa-token 会自动验证Token
    // 如果Token无效，会返回401

    // 获取登录用户ID
    let login_id = StpUtil::get_login_id()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 继续处理请求
    Ok(next.run(request).await)
}

/// 权限检查中间件
pub fn require_permission(permission: &'static str) -> impl Fn(Request, Next) -> Response + Clone {
    move |request: Request, next: Next| {
        let permission = permission;

        async move {
            // 检查权限
            StpUtil::check_permission(permission)
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;

            Ok(next.run(request).await)
        }
    }
}

/// 角色检查中间件
pub fn require_role(role: &'static str) -> impl Fn(Request, Next) -> Response + Clone {
    move |request: Request, next: Next| {
        let role = role;

        async move {
            // 检查角色
            StpUtil::check_role(role)
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;

            Ok(next.run(request).await)
        }
    }
}
```

### 4.4 gRPC 拦截器

```rust
// crates/admin-common/src/auth/interceptor.rs

use tonic::{Request, Status, service::Interceptor};
use sa_token_plugin_axum::prelude::*;

/// gRPC 认证拦截器
pub struct AuthInterceptor;

impl Interceptor for AuthInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        // 从metadata获取Token
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization token"))?;

        // 验证Token并获取登录ID
        let login_id = StpUtil::get_login_id_by_token(token)
            .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;

        // 将登录ID添加到请求扩展
        let mut request = request;
        request.extensions_mut().insert(login_id);

        Ok(request)
    }
}

/// 权限检查拦截器
pub struct PermissionInterceptor {
    permission: &'static str,
}

impl PermissionInterceptor {
    pub fn new(permission: &'static str) -> Self {
        Self { permission }
    }
}

impl Interceptor for PermissionInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let login_id = request
            .extensions()
            .get::<u64>()
            .ok_or_else(|| Status::unauthenticated("Login ID not found"))?;

        // 检查权限
        StpUtil::check_permission_by_login_id(*login_id, self.permission)
            .map_err(|_| Status::permission_denied("Insufficient permissions"))?;

        Ok(request)
    }
}
```

### 4.5 权限和角色管理

```rust
// crates/admin-application/src/auth/service.rs

use sa_token_plugin_axum::prelude::*;

/// 认证应用服务
pub struct AuthAppService {
    user_service: UserService<impl UserRepository>,
    role_service: RoleService<impl RoleRepository>,
    permission_service: PermissionService<impl PermissionRepository>,
}

impl AuthAppService {
    /// 用户登录
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip: &str,
    ) -> Result<LoginResponse, AuthError> {
        // 验证用户名密码
        let user = self.user_service
            .validate_login(username, password, ip)
            .await
            .map_err(|e| match e {
                UserServiceError::UserNotFound => AuthError::InvalidCredentials,
                UserServiceError::IncorrectPassword => AuthError::InvalidCredentials,
                UserServiceError::UserDisabled => AuthError::UserDisabled,
                UserServiceError::UserLocked => AuthError::UserLocked,
                UserServiceError::UserLockedAfterAttempts => AuthError::UserLocked,
                UserServiceError::PasswordExpired => AuthError::PasswordExpired,
                _ => AuthError::InternalError,
            })?;

        // 使用 sa-token 登录
        let token = StpUtil::login(user.id())
            .await
            .map_err(|_| AuthError::InternalError)?;

        // 设置用户权限
        let permissions = self.permission_service
            .get_user_permissions(user.id())
            .await
            .unwrap_or_default();
        StpUtil::set_permissions(&permissions)
            .await
            .map_err(|_| AuthError::InternalError)?;

        // 设置用户角色
        let roles = self.role_service
            .get_user_roles(user.id())
            .await
            .unwrap_or_default();
        StpUtil::set_roles(&roles)
            .await
            .map_err(|_| AuthError::InternalError)?;

        Ok(LoginResponse {
            token,
            user_id: user.id(),
            username: user.username().to_string(),
        })
    }

    /// 用户登出
    pub async fn logout(&self) -> Result<(), AuthError> {
        StpUtil::logout()
            .await
            .map_err(|_| AuthError::InternalError)?;
        Ok(())
    }

    /// 获取当前用户信息
    pub async fn get_current_user(&self) -> Result<CurrentUserResponse, AuthError> {
        let login_id = StpUtil::get_login_id()
            .await
            .map_err(|_| AuthError::NotLoggedIn)?;

        let user = self.user_service
            .get_user(login_id as u64)
            .await
            .map_err(|_| AuthError::UserNotFound)?;

        let permissions = StpUtil::get_permission_list()
            .await
            .unwrap_or_default();

        let roles = StpUtil::get_role_list()
            .await
            .unwrap_or_default();

        Ok(CurrentUserResponse {
            id: user.id(),
            username: user.username().to_string(),
            real_name: user.profile().real_name.clone(),
            email: user.profile().email.clone(),
            phone: user.profile().phone.clone(),
            avatar: user.profile().avatar.clone(),
            permissions,
            roles,
        })
    }

    /// 刷新Token
    pub async fn refresh_token(&self) -> Result<LoginResponse, AuthError> {
        // sa-token 会自动处理Token续期
        let login_id = StpUtil::get_login_id()
            .await
            .map_err(|_| AuthError::NotLoggedIn)?;

        // 获取新Token
        let token = StpUtil::login(login_id)
            .await
            .map_err(|_| AuthError::InternalError)?;

        Ok(LoginResponse {
            token,
            user_id: login_id as u64,
            username: String::new(), // 需要查询用户名
        })
    }
}

/// 登录响应
pub struct LoginResponse {
    pub token: String,
    pub user_id: u64,
    pub username: String,
}

/// 当前用户响应
pub struct CurrentUserResponse {
    pub id: u64,
    pub username: String,
    pub real_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub permissions: Vec<String>,
    pub roles: Vec<String>,
}

/// 认证错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User disabled")]
    UserDisabled,

    #[error("User locked")]
    UserLocked,

    #[error("Password expired")]
    PasswordExpired,

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("User not found")]
    UserNotFound,

    #[error("Internal error")]
    InternalError,
}
```

### 4.6 使用示例

```rust
// crates/admin-server/src/grpc/user.rs

use tonic::{Request, Response, Status};
use admin_proto::admin::user_service_server::UserService;
use admin_proto::admin::*;
use sa_token_plugin_axum::prelude::*;

pub struct UserServiceImpl {
    user_app_service: UserAppService,
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    /// 创建用户 - 使用过程宏进行权限检查
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        // 获取当前登录用户ID
        let login_id = StpUtil::get_login_id()
            .await
            .map_err(|_| Status::unauthenticated("Not logged in"))?;

        // 检查权限
        StpUtil::check_permission("user:create")
            .await
            .map_err(|_| Status::permission_denied("No permission to create user"))?;

        let req = request.into_inner();

        // 调用应用服务
        let user = self.user_app_service
            .create_user(
                req.username,
                req.real_name,
                req.email,
                req.phone,
                req.nickname,
                req.gender,
                req.remark,
                req.role_ids,
                req.department_id,
                login_id as u64,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to create user: {}", e)))?;

        // 转换为响应
        let response = UserResponse {
            id: user.id(),
            username: user.username().to_string(),
            // ... 其他字段
        };

        Ok(Response::new(response))
    }
}
```

### 4.7 事件监听

```rust
// crates/admin-common/src/auth/event.rs

use sa_token_plugin_axum::prelude::*;

/// sa-token 事件监听器
pub struct SaTokenEventListener;

#[async_trait]
impl SaTokenListener for SaTokenEventListener {
    /// 登录事件
    async fn do_login(&self, login_id: &str, token: &str) {
        tracing::info!("User logged in: {}", login_id);
        // 记录登录日志
    }

    /// 登出事件
    async fn do_logout(&self, login_id: &str, token: &str) {
        tracing::info!("User logged out: {}", login_id);
        // 记录登出日志
    }

    /// 被踢下线事件
    async fn do_kickout(&self, login_id: &str, token: &str) {
        tracing::info!("User kicked out: {}", login_id);
        // 记录踢出日志
    }

    /// 被顶下线事件
    async fn do_replaced(&self, login_id: &str, token: &str) {
        tracing::info!("User replaced: {}", login_id);
        // 记录顶替日志
    }
}

/// 注册事件监听器
pub fn register_event_listener() {
    StpUtil::register_listener(SaTokenEventListener);
}
```

### 4.8 路由配置

```rust
// crates/admin-server/src/startup.rs

use axum::{Router, middleware};
use sa_token_plugin_axum::prelude::*;

pub async fn create_router(sa_token_state: SaTokenState) -> Router {
    // 公开路由（不需要认证）
    let public_routes = Router::new()
        .route("/auth/login", axum::routing::post(login))
        .route("/health", axum::routing::get(health_check));

    // 需要认证的路由
    let authenticated_routes = Router::new()
        .route("/user/info", axum::routing::get(get_user_info))
        .route("/user/list", axum::routing::get(list_users))
        .route("/user/create", axum::routing::post(create_user))
        .route_layer(middleware::from_fn(auth_middleware));

    // 需要特定权限的路由
    let admin_routes = Router::new()
        .route("/user/delete", axum::routing::delete(delete_user))
        .route_layer(middleware::from_fn(require_permission("user:delete")));

    // 组合路由
    Router::new()
        .merge(public_routes)
        .merge(authenticated_routes)
        .merge(admin_routes)
        .layer(SaTokenMiddleware::new(sa_token_state))
}
```

---

## 五、数据权限设计

### 5.1 数据权限范围

| 范围 | 说明 | SQL条件示例 |
|------|------|-------------|
| 全部数据 | 不过滤 | 无 |
| 自定义数据 | 指定部门 | dept_id IN (1,2,3) |
| 本部门数据 | 只看本部门 | dept_id = 1 |
| 本部门及以下 | 本部门+子部门 | dept_id IN (SELECT id FROM dept WHERE hierarchy LIKE '1/%') |
| 仅本人数据 | 只看自己 | create_by = 123 |

### 5.2 数据权限实现

```rust
// crates/admin-common/src/auth/data_scope.rs

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// 数据权限范围
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataScope {
    All = 1,              // 全部数据
    Custom = 2,           // 自定义数据
    Department = 3,       // 本部门数据
    DeptAndChildren = 4,  // 本部门及以下
    SelfOnly = 5,         // 仅本人数据
}

/// 数据权限注解
pub struct DataScopeFilter {
    pub dept_alias: String,
    pub user_alias: String,
    pub scope: DataScope,
}

/// 数据权限中间件
pub async fn data_scope_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 获取当前用户的数据权限范围
    let login_id = StpUtil::get_login_id()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 查询用户的数据权限范围
    let data_scope = get_user_data_scope(login_id as u64)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 将数据权限添加到请求扩展
    let mut request = request;
    request.extensions_mut().insert(data_scope);

    Ok(next.run(request).await)
}

async fn get_user_data_scope(user_id: u64) -> Result<DataScope, Box<dyn std::error::Error>> {
    // 查询用户角色的数据权限范围
    // 这里简化处理，实际需要查询数据库
    Ok(DataScope::All)
}
```

---

## 六、缓存策略设计

### 6.1 缓存架构

| 数据类型 | 缓存策略 | 过期时间 | 更新方式 |
|----------|----------|----------|----------|
| 用户权限 | sa-token管理 | 跟随Token | 登录时设置 |
| 菜单数据 | Redis | 1小时 | 写时删除 |
| 字典数据 | Redis | 24小时 | 写时删除 |
| 系统配置 | Redis | 24小时 | 写时删除 |

---

## 七、gRPC API设计规范

### 7.1 Proto文件定义

```protobuf
// proto/admin/auth.proto

syntax = "proto3";

package admin;

import "google/protobuf/timestamp.proto";
import "common/common.proto";

// 认证服务
service AuthService {
    // 登录
    rpc Login(LoginRequest) returns (LoginResponse);

    // 退出登录
    rpc Logout(LogoutRequest) returns (common.Empty);

    // 刷新Token
    rpc RefreshToken(common.Empty) returns (LoginResponse);

    // 获取当前用户信息
    rpc GetCurrentUser(common.Empty) returns (CurrentUserResponse);

    // 获取当前用户权限
    rpc GetCurrentUserPermissions(common.Empty) returns (GetPermissionsResponse);

    // 获取当前用户角色
    rpc GetCurrentUserRoles(common.Empty) returns (GetRolesResponse);
}

// 登录请求
message LoginRequest {
    string username = 1;
    string password = 2;
    optional string captcha_code = 3;
    optional string captcha_key = 4;
}

// 登录响应
message LoginResponse {
    string token = 1;
    uint64 user_id = 2;
    string username = 3;
}

// 退出登录请求
message LogoutRequest {
    string token = 1;
}

// 当前用户响应
message CurrentUserResponse {
    uint64 id = 1;
    string username = 2;
    optional string real_name = 3;
    optional string email = 4;
    optional string phone = 5;
    optional string nickname = 6;
    optional string avatar = 7;
    repeated string permissions = 8;
    repeated string roles = 9;
}

// 获取权限响应
message GetPermissionsResponse {
    repeated string permissions = 1;
}

// 获取角色响应
message GetRolesResponse {
    repeated string roles = 1;
}
```

```protobuf
// proto/admin/user.proto

syntax = "proto3";

package admin;

import "google/protobuf/timestamp.proto";
import "google/protobuf/wrappers.proto";
import "common/common.proto";

// 用户服务
service UserService {
    rpc CreateUser(CreateUserRequest) returns (UserResponse);
    rpc UpdateUser(UpdateUserRequest) returns (UserResponse);
    rpc DeleteUser(DeleteUserRequest) returns (common.Empty);
    rpc GetUser(GetUserRequest) returns (UserResponse);
    rpc ListUsers(ListUsersRequest) returns (ListUsersResponse);
    rpc ChangePassword(ChangePasswordRequest) returns (common.Empty);
    rpc ResetPassword(ResetPasswordRequest) returns (ResetPasswordResponse);
    rpc UpdateUserStatus(UpdateUserStatusRequest) returns (common.Empty);
    rpc AssignRoles(AssignRolesRequest) returns (common.Empty);
    rpc AssignDepartment(AssignDepartmentRequest) returns (common.Empty);
}

// ID使用uint64
message CreateUserRequest {
    string username = 1;
    string real_name = 2;
    optional string email = 3;
    optional string phone = 4;
    optional string nickname = 5;
    Gender gender = 6;
    optional string remark = 7;
    repeated uint64 role_ids = 8;
    optional uint64 department_id = 9;
}

message UpdateUserRequest {
    uint64 id = 1;
    optional string real_name = 2;
    optional string email = 3;
    optional string phone = 4;
    optional string nickname = 5;
    optional Gender gender = 6;
    optional string remark = 7;
    optional string avatar = 8;
}

message DeleteUserRequest {
    uint64 id = 1;
}

message GetUserRequest {
    uint64 id = 1;
}

message ListUsersRequest {
    optional string username = 1;
    optional string real_name = 2;
    optional string phone = 3;
    optional string email = 4;
    optional UserStatus status = 5;
    optional uint64 department_id = 6;
    optional uint64 role_id = 7;
    optional google.protobuf.Timestamp created_start = 8;
    optional google.protobuf.Timestamp created_end = 9;
    int32 page = 10;
    int32 page_size = 11;
    string sort_by = 12;
    bool sort_desc = 13;
}

message ListUsersResponse {
    repeated UserResponse users = 1;
    int64 total = 2;
    int32 page = 3;
    int32 page_size = 4;
}

message UserResponse {
    uint64 id = 1;
    string username = 2;
    string real_name = 3;
    optional string email = 4;
    optional string phone = 5;
    optional string nickname = 6;
    Gender gender = 7;
    optional string avatar = 8;
    optional string remark = 9;
    UserStatus status = 10;
    repeated RoleInfo roles = 11;
    optional DepartmentInfo department = 12;
    google.protobuf.Timestamp created_at = 13;
    optional google.protobuf.Timestamp last_login_at = 14;
    optional string last_login_ip = 15;
}

message ChangePasswordRequest {
    uint64 user_id = 1;
    string old_password = 2;
    string new_password = 3;
}

message ResetPasswordRequest {
    uint64 user_id = 1;
}

message ResetPasswordResponse {
    string new_password = 1;
}

message UpdateUserStatusRequest {
    uint64 user_id = 1;
    UserStatus status = 2;
}

message AssignRolesRequest {
    uint64 user_id = 1;
    repeated uint64 role_ids = 2;
}

message AssignDepartmentRequest {
    uint64 user_id = 1;
    uint64 department_id = 2;
}

enum Gender {
    GENDER_UNKNOWN = 0;
    GENDER_MALE = 1;
    GENDER_FEMALE = 2;
}

enum UserStatus {
    USER_STATUS_ACTIVE = 0;
    USER_STATUS_DISABLED = 1;
    USER_STATUS_LOCKED = 2;
}

message RoleInfo {
    uint64 id = 1;
    string name = 2;
    string code = 3;
}

message DepartmentInfo {
    uint64 id = 1;
    string name = 2;
    optional uint64 parent_id = 3;
}
```

---

## 八、安全设计

### 8.1 安全防护

| 威胁 | 防护措施 | 实现方式 |
|------|----------|----------|
| SQL注入 | 参数化查询 | SQLx编译时检查 |
| XSS攻击 | 输入过滤 | HTML转义 |
| CSRF攻击 | Token验证 | sa-token管理 |
| 暴力破解 | 登录失败锁定 | 用户领域服务 |
| 会话劫持 | Token管理 | sa-token |
| 越权访问 | 权限检查 | sa-token权限/角色 |
| 敏感数据 | 加密存储 | Argon2密码哈希 |
| 接口滥用 | 限流 | Tower中间件 |

---

## 九、性能优化

### 9.1 Rust特有优化

1. **零成本抽象**：使用trait和泛型，编译时单态化
2. **内存安全**：借用检查器避免数据竞争
3. **异步并发**：Tokio高效异步运行时
4. **无GC**：确定性内存管理，无停顿

### 9.2 编译优化

```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## 十、监控与告警

```rust
// crates/admin-server/src/startup.rs

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

---

## 十一、部署架构

### Docker部署

```dockerfile
# docker/Dockerfile

FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/admin-server /usr/local/bin/

EXPOSE 50051 8080

CMD ["admin-server"]
```

```yaml
# docker/docker-compose.yml

version: '3.8'

services:
  admin-server:
    build:
      context: ..
      dockerfile: docker/Dockerfile
    ports:
      - "50051:50051"
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://admin:password@postgres:5432/admin
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      - postgres
      - redis

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=admin
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=admin
    volumes:
      - postgres-data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/conf.d:/etc/nginx/conf.d
    depends_on:
      - admin-server

volumes:
  postgres-data:
  redis-data:
```

---

## 十二、扩展性设计

### 多租户支持

- 采用共享数据库、tenant_id隔离方案
- SQLx查询时自动添加租户条件
- 支持租户级别的配置和数据隔离

---

## 十三、数据库设计

```sql
-- migrations/001_create_users.sql
-- 所有ID使用BIGINT (对应Rust的u64)

CREATE TABLE sys_user (
    id BIGINT PRIMARY KEY,  -- 雪花算法生成
    username VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    real_name VARCHAR(50),
    nickname VARCHAR(50),
    email VARCHAR(100),
    phone VARCHAR(20),
    gender SMALLINT DEFAULT 0,
    avatar VARCHAR(255),
    dept_id BIGINT,
    status SMALLINT DEFAULT 1,
    login_attempts INT DEFAULT 0,
    last_login_time TIMESTAMPTZ,
    last_login_ip VARCHAR(50),
    password_update_time TIMESTAMPTZ,
    remark VARCHAR(500),
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE,
    tenant_id BIGINT DEFAULT 0
);

CREATE INDEX idx_user_username ON sys_user(username);
CREATE INDEX idx_user_email ON sys_user(email);
CREATE INDEX idx_user_phone ON sys_user(phone);
CREATE INDEX idx_user_dept_id ON sys_user(dept_id);
CREATE INDEX idx_user_status ON sys_user(status);

-- 角色表
CREATE TABLE sys_role (
    id BIGINT PRIMARY KEY,
    role_name VARCHAR(50) NOT NULL,
    role_code VARCHAR(50) NOT NULL UNIQUE,
    description VARCHAR(255),
    data_scope SMALLINT DEFAULT 1,
    sort INT DEFAULT 0,
    status SMALLINT DEFAULT 1,
    remark VARCHAR(500),
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE,
    tenant_id BIGINT DEFAULT 0
);

-- 权限表
CREATE TABLE sys_permission (
    id BIGINT PRIMARY KEY,
    permission_code VARCHAR(100) NOT NULL UNIQUE,
    permission_name VARCHAR(50) NOT NULL,
    permission_type SMALLINT NOT NULL,
    parent_id BIGINT DEFAULT 0,
    resource VARCHAR(255),
    api_method VARCHAR(10),
    sort INT DEFAULT 0,
    description VARCHAR(255),
    status SMALLINT DEFAULT 1,
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE
);

-- 菜单表
CREATE TABLE sys_menu (
    id BIGINT PRIMARY KEY,
    menu_name VARCHAR(50) NOT NULL,
    parent_id BIGINT DEFAULT 0,
    sort INT DEFAULT 0,
    path VARCHAR(255),
    component VARCHAR(255),
    menu_type SMALLINT NOT NULL,
    visible BOOLEAN DEFAULT TRUE,
    status SMALLINT DEFAULT 1,
    perms VARCHAR(100),
    icon VARCHAR(100),
    is_external BOOLEAN DEFAULT FALSE,
    is_cache BOOLEAN DEFAULT FALSE,
    remark VARCHAR(500),
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE
);

-- 用户角色关联表
CREATE TABLE sys_user_role (
    id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, role_id)
);

-- 角色权限关联表
CREATE TABLE sys_role_permission (
    id BIGINT PRIMARY KEY,
    role_id BIGINT NOT NULL,
    permission_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(role_id, permission_id)
);

-- 角色菜单关联表
CREATE TABLE sys_role_menu (
    id BIGINT PRIMARY KEY,
    role_id BIGINT NOT NULL,
    menu_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(role_id, menu_id)
);

-- 部门表
CREATE TABLE sys_dept (
    id BIGINT PRIMARY KEY,
    dept_name VARCHAR(50) NOT NULL,
    dept_code VARCHAR(50),
    parent_id BIGINT DEFAULT 0,
    hierarchy VARCHAR(255),
    level INT DEFAULT 1,
    leader_id BIGINT,
    phone VARCHAR(20),
    email VARCHAR(100),
    sort INT DEFAULT 0,
    status SMALLINT DEFAULT 1,
    remark VARCHAR(500),
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE,
    tenant_id BIGINT DEFAULT 0
);

-- 文件表
CREATE TABLE sys_file (
    id BIGINT PRIMARY KEY,
    file_name VARCHAR(255) NOT NULL,
    storage_path VARCHAR(500) NOT NULL,
    content_type VARCHAR(100),
    size BIGINT,
    md5_hash VARCHAR(32),
    uploader_id BIGINT,
    status SMALLINT DEFAULT 1,
    business_type VARCHAR(50),
    business_id VARCHAR(50),
    upload_time TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    expire_time TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE
);

-- 系统配置表
CREATE TABLE sys_config (
    id BIGINT PRIMARY KEY,
    config_key VARCHAR(100) NOT NULL UNIQUE,
    config_value TEXT,
    value_type VARCHAR(20) DEFAULT 'STRING',
    group_code VARCHAR(50),
    description VARCHAR(255),
    enabled BOOLEAN DEFAULT TRUE,
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE
);

-- 字典类型表
CREATE TABLE sys_dict_type (
    id BIGINT PRIMARY KEY,
    dict_code VARCHAR(100) NOT NULL UNIQUE,
    dict_name VARCHAR(100) NOT NULL,
    remark VARCHAR(500),
    enabled BOOLEAN DEFAULT TRUE,
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE
);

-- 字典项表
CREATE TABLE sys_dict_item (
    id BIGINT PRIMARY KEY,
    dict_type_id BIGINT NOT NULL,
    item_key VARCHAR(100) NOT NULL,
    item_value VARCHAR(255),
    label VARCHAR(100),
    css_class VARCHAR(50),
    parent_item_id BIGINT,
    sort INT DEFAULT 0,
    remark VARCHAR(500),
    enabled BOOLEAN DEFAULT TRUE,
    created_by BIGINT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_by BIGINT,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    deleted BOOLEAN DEFAULT FALSE,
    UNIQUE(dict_type_id, item_key)
);

-- 操作日志表
CREATE TABLE sys_operation_log (
    id BIGINT PRIMARY KEY,
    user_id BIGINT,
    username VARCHAR(50),
    module VARCHAR(50),
    action VARCHAR(50),
    target_type VARCHAR(50),
    target_id VARCHAR(50),
    request_url VARCHAR(255),
    request_method VARCHAR(10),
    request_data TEXT,
    response_data TEXT,
    ip_address VARCHAR(50),
    user_agent VARCHAR(500),
    result SMALLINT DEFAULT 1,
    duration BIGINT,
    exception TEXT,
    "timestamp" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 登录日志表
CREATE TABLE sys_login_log (
    id BIGINT PRIMARY KEY,
    user_id BIGINT,
    username VARCHAR(50),
    ip_address VARCHAR(50),
    location VARCHAR(100),
    browser VARCHAR(100),
    os VARCHAR(100),
    device VARCHAR(100),
    result SMALLINT DEFAULT 1,
    message VARCHAR(255),
    login_time TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 角色数据权限关联表
CREATE TABLE sys_role_dept (
    id BIGINT PRIMARY KEY,
    role_id BIGINT NOT NULL,
    dept_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(role_id, dept_id)
);
```

---

## 十四、附录

### 依赖配置

```toml
# Cargo.toml

[workspace]
members = [
    "crates/admin-proto",
    "crates/admin-common",
    "crates/admin-domain",
    "crates/admin-application",
    "crates/admin-infrastructure",
    "crates/admin-server",
]

[workspace.dependencies]
# Web框架
axum = "0.8"
tonic = "0.12"

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 认证授权
sa-token-plugin-axum = { version = "0.1.14", features = ["redis"] }

# 数据库
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio"] }

# Redis
redis = { version = "0.25", features = ["tokio-comp"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"
prost = "0.13"

# 错误处理
anyhow = "1"
thiserror = "2"

# 异步trait
async-trait = "0.1"

# 时间
chrono = { version = "0.4", features = ["serde"] }

# UUID（仅用于非ID场景）
uuid = { version = "1", features = ["v4", "serde"] }

# 密码哈希
argon2 = "0.5"

# 验证
validator = { version = "0.18", features = ["derive"] }

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# 正则
regex = "1"

# 随机数
rand = "0.8"

# 懒加载
lazy_static = "1"
```

### 开发规范

1. **ID规范**：所有ID统一使用 `u64` 类型，使用雪花算法生成
2. **认证授权**：使用 sa-token-rust，不要自定义实现
3. **错误处理**：使用 `thiserror` 定义领域错误，`anyhow` 处理应用错误
4. **异步编程**：统一使用 `async/await`，避免阻塞
5. **序列化**：使用 `serde` 进行JSON序列化，`prost` 进行Protobuf序列化

---

**文档结束**

> 本文档定义了使用 Rust + Axum + gRPC + sa-token-rust 实现的通用后台管理系统完整设计。
