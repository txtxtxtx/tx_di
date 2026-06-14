use jiff::Timestamp; // 引入jiff库中的Timestamp类型，用于处理时间
use serde::{Deserialize, Serialize}; // 引入serde库中的Deserialize和Serialize trait，用于序列化和反序列化

use crate::shared::model::{AggregateRoot, AuditFields, DomainEvent}; // 引入共享模块中的相关模型和trait
use crate::AggregateRoot;
use crate::shared::model::value_object::{DeletedStatus, TenantId};
use crate::user::model::value_object::{Sex, UserStatus};
// 引入重新导出的派生宏

/// User aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, AggregateRoot)]
/// 用户实体结构体，用于存储用户的基本信息、状态、权限等相关数据
pub struct User {
    /// 用户唯一标识ID
    pub id: u64,
    /// 用户名，用于登录系统的账号
    pub username: String,
    /// 用户密码，经过加密处理的字符串
    pub password: String,
    /// 用户昵称，显示给其他用户看的名称
    pub nickname: String,
    /// 备注信息，可选字段
    pub remark: Option<String>,
    /// 用户电子邮箱，可选字段
    pub email: Option<String>,
    /// 用户手机号码，可选字段
    pub mobile: Option<String>,
    /// 用户性别
    pub sex: Sex,
    /// 用户头像URL，可选字段
    pub avatar: Option<String>,
    /// 用户状态：Active(正常) / Disabled(禁用) / Locked(锁定)
    pub status: UserStatus,
    /// 用户最近登录的IP地址，可选字段
    pub login_ip: Option<String>,
    /// 用户最近登录的时间，可选字段，使用UTC时间
    pub login_date: Option<Timestamp>,
    /// 租户ID，用于多租户系统中的租户隔离
    pub tenant_id: TenantId,
    /// 审计字段，包含创建、修改等信息
    pub audit: AuditFields,
    /// 用户拥有的角色ID列表
    pub role_ids: Vec<u64>,
    /// 用户所属的部门ID列表
    pub dept_ids: Vec<u64>,
    // 领域事件列表，不对外公开，用于领域事件处理
    events: Vec<DomainEvent>,
}

impl User {
    /// 从持久化层恢复用户（不触发领域事件）
    pub fn restore(
        id: u64,
        username: String,
        password: String,
        nickname: String,
        remark: Option<String>,
        email: Option<String>,
        mobile: Option<String>,
        sex: Sex,
        avatar: Option<String>,
        status: UserStatus,
        login_ip: Option<String>,
        login_date: Option<Timestamp>,
        tenant_id: TenantId,
        audit: AuditFields,
        role_ids: Vec<u64>,
        dept_ids: Vec<u64>,
    ) -> Self {
        Self {
            id,
            username,
            password,
            nickname,
            remark,
            email,
            mobile,
            sex,
            avatar,
            status,
            login_ip,
            login_date,
            tenant_id,
            audit,
            role_ids,
            dept_ids,
            events: Vec::new(),
        }
    }

    /// Create a new user
    pub fn create(
        id: u64,
        username: String,
        password: String,
        nickname: String,
        creator: Option<String>,
    ) -> Self {
        let mut user = Self {
            id,
            username: username.clone(),
            password,
            nickname,
            remark: None,
            email: None,
            mobile: None,
            sex: Sex::Unknown,
            avatar: None,
            status: UserStatus::Active,
            login_ip: None,
            login_date: None,
            tenant_id: TenantId::default(),
            audit: AuditFields {
                creator: creator.clone(),
                create_time: Timestamp::now(),
                updater: creator,
                update_time: Timestamp::now(),
                deleted: DeletedStatus::Normal,
            },
            role_ids: Vec::new(),
            dept_ids: Vec::new(),
            events: Vec::new(),
        };
        user.add_event(DomainEvent::UserCreated {
            user_id: id,
            username,
        });
        user
    }

    /// Set basic info
    pub fn set_basic_info(
        &mut self,
        nickname: String,
        email: Option<String>,
        mobile: Option<String>,
        sex: Sex,
        remark: Option<String>,
        updater: Option<String>,
    ) {
        self.nickname = nickname;
        self.email = email;
        self.mobile = mobile;
        self.sex = sex;
        self.remark = remark;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::UserUpdated { user_id: self.id });
    }

    /// Change status
    pub fn change_status(&mut self, status: UserStatus, updater: Option<String>) {
        self.status = status;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::UserStatusChanged {
            user_id: self.id,
            status,
        });
    }

    /// Change password
    pub fn change_password(&mut self, password: String, updater: Option<String>) {
        self.password = password;
        self.audit.updater = updater;
        self.audit.update_time = Timestamp::now();
        self.add_event(DomainEvent::UserPasswordChanged { user_id: self.id });
    }

    /// Record login
    pub fn record_login(&mut self, ip: String) {
        self.login_ip = Some(ip.clone());
        self.login_date = Some(Timestamp::now());
        self.add_event(DomainEvent::UserLoggedIn {
            user_id: self.id,
            ip,
        });
    }

    /// Set roles
    pub fn set_roles(&mut self, role_ids: Vec<u64>) {
        self.role_ids = role_ids;
    }

    /// Set departments
    pub fn set_departments(&mut self, dept_ids: Vec<u64>) {
        self.dept_ids = dept_ids;
    }

    /// Soft delete
    pub fn soft_delete(&mut self, updater: Option<String>) {
        self.audit.delete(updater);
        self.add_event(DomainEvent::UserDeleted { user_id: self.id });
    }

    /// Check if user is active
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active && self.audit.deleted == DeletedStatus::Normal
    }

    /// Check if user is locked
    pub fn is_locked(&self) -> bool {
        self.status == UserStatus::Locked
    }
}
