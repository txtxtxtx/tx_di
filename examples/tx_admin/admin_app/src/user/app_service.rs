use std::sync::Arc;

use crate::user::dto::*;
use admin_domain::department::repository::DepartmentRepository;
use admin_domain::menu::repository::MenuRepository;
use admin_domain::role::repository::RoleRepository;
use admin_domain::shared::event_publisher::DomainEventPublisher;
use admin_domain::shared::model::AggregateRoot;
use admin_domain::shared::repository::RepositoryError;
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{LoginUser, Sex, UserQuery, UserStatus};
use admin_domain::user::service::UserService;
use admin_proto::{ChangePasswordRequest, CreateUserRequest, ListUsersRequest, UpdateUserRequest};
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;

/// User application service - 编排领域操作 + 跨聚合校验
#[derive(Component)]
pub struct UserAppService {
    user_service: Arc<UserService>,
    role_repo: Arc<dyn RoleRepository>,
    dept_repo: Arc<dyn DepartmentRepository>,
    menu_repo: Arc<dyn MenuRepository>,
    /// 领域事件发布器（事务提交后发布聚合根事件；未注册时跳过）
    event_publisher: Option<Arc<dyn DomainEventPublisher>>,
}

impl UserAppService {
    /// 创建用户应用服务实例
    ///
    /// # 参数
    /// - `user_service` - 用户领域服务
    /// - `role_repo` - 角色仓库（用于跨聚合校验）
    /// - `dept_repo` - 部门仓库（用于跨聚合校验）
    /// - `menu_repo` - 菜单仓库（用于构建登录用户权限）
    pub fn new(
        user_service: Arc<UserService>,
        role_repo: Arc<dyn RoleRepository>,
        dept_repo: Arc<dyn DepartmentRepository>,
        menu_repo: Arc<dyn MenuRepository>,
    ) -> Self {
        Self {
            user_service,
            role_repo,
            dept_repo,
            menu_repo,
            event_publisher: None,
        }
    }

    /// 发布聚合根待处理事件（事务提交后调用）
    fn publish_events(&self, aggregate: &mut User) {
        if self.event_publisher.is_none() {
            return;
        }
        let events = aggregate.events().to_vec();
        aggregate.clear_events();
        if let Some(publisher) = &self.event_publisher {
            publisher.publish(events);
        }
    }

    /// 获取 UserService 引用（供 AuthAppService 等编排者使用）
    pub fn user_service(&self) -> &Arc<UserService> {
        &self.user_service
    }

    /// 创建新用户
    pub async fn create_user(
        &self,
        req: CreateUserRequest,
        creator: Option<String>,
    ) -> AppResult<UserResponse> {
        let email = req.email.filter(|s| !s.is_empty());
        let mobile = req.mobile.filter(|s| !s.is_empty());
        let remark = req.remark.filter(|s| !s.is_empty());
        let sex = req.sex.map(Sex::from).unwrap_or_default();

        // Check email uniqueness
        if let Some(ref e) = email
            && self.user_service.exists_by_email(e).await?
        {
            Err(RepositoryError::DuplicateEmail)?;
        }

        // Check mobile uniqueness
        if let Some(ref m) = mobile
            && self.user_service.exists_by_mobile(m).await?
        {
            Err(RepositoryError::DuplicateMobile)?;
        }

        // 构建领域对象（密码哈希等，不落库）
        let mut user = self
            .user_service
            .prepare_create(req.username, req.password, req.nickname, creator.clone())
            .await?;

        // 设置可选字段（更新审计信息）
        if email.is_some() || mobile.is_some() || req.sex.is_some() || remark.is_some() {
            user.set_basic_info(
                user.nickname.clone(),
                email.clone(),
                mobile.clone(),
                sex,
                remark.clone(),
                creator.clone(),
            );
        }

        // 原子提交：建用户 + 绑角色 + 绑部门在同一个数据库事务中完成
        let role_ids = req.role_ids.clone();
        let dept_ids = req.dept_ids.clone();
        self.user_service
            .user_repo()
            .create_user_with_bindings(&user, &role_ids, &dept_ids)
            .await?;
        user.role_ids = role_ids;
        user.dept_ids = dept_ids;

        // 事务提交后发布领域事件（UserCreated 等）
        self.publish_events(&mut user);

        Ok(user_to_response(user))
    }

    /// 更新用户信息
    pub async fn update_user(
        &self,
        req: UpdateUserRequest,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self
            .user_service
            .update_user(
                req.user_id,
                req.nickname.unwrap_or_default(),
                req.email.filter(|s| !s.is_empty()),
                req.mobile.filter(|s| !s.is_empty()),
                req.sex.map(Sex::from).unwrap_or_default(),
                req.remark.filter(|s| !s.is_empty()),
                updater,
            )
            .await?;
        Ok(user_to_response(user))
    }

    /// 删除用户
    pub async fn delete_user(&self, user_id: u64, updater: Option<String>) -> AppResult<()> {
        self.user_service.delete_user(user_id, updater).await
    }

    /// 变更用户状态
    pub async fn change_status(
        &self,
        user_id: u64,
        status: UserStatus,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self
            .user_service
            .change_status(user_id, status, updater)
            .await?;
        Ok(user_to_response(user))
    }

    /// 修改用户密码
    pub async fn change_password(
        &self,
        req: ChangePasswordRequest,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service
            .change_password(req.user_id, req.new_password, updater)
            .await?;
        Ok(())
    }

    /// 为用户分配角色（跨聚合校验：校验角色存在且启用）
    pub async fn assign_roles(&self, user_id: u64, role_ids: Vec<u64>) -> AppResult<()> {
        let user = self.user_service.get_user(user_id).await?;

        // 用户必须为 Active 状态
        if user.status != UserStatus::Active {
            Err(RepositoryError::ValidationUserStatus)?;
        }

        // 校验每个角色存在且为启用状态（status == 0 即 Enabled）
        let roles = self.role_repo.find_by_ids(&role_ids).await?;
        // 数据完整性校验：输入 ID 与命中记录数必须一致，防止悬空引用
        if roles.len() != role_ids.len() {
            Err(RepositoryError::NotFoundRole)?;
        }
        for r in &roles {
            if r.status != 0 {
                Err(RepositoryError::ValidationUserStatus)?;
            }
        }

        self.user_service
            .user_repo()
            .bind_roles(user_id, &role_ids)
            .await?;
        Ok(())
    }

    /// 为用户分配部门（跨聚合校验：校验部门存在且启用）
    pub async fn assign_departments(&self, user_id: u64, dept_ids: Vec<u64>) -> AppResult<()> {
        let user = self.user_service.get_user(user_id).await?;

        // 用户必须为 Active 状态
        if user.status != UserStatus::Active {
            Err(RepositoryError::ValidationUserStatus)?;
        }

        // 校验每个部门存在且为启用状态
        let depts = self.dept_repo.find_by_ids(&dept_ids).await?;
        // 数据完整性校验：输入 ID 与命中记录数必须一致，防止悬空引用
        if depts.len() != dept_ids.len() {
            Err(RepositoryError::NotFoundDept)?;
        }
        for d in &depts {
            if d.status != 0 {
                Err(RepositoryError::ValidationDeptDisabled)?;
            }
        }

        self.user_service
            .user_repo()
            .bind_departments(user_id, &dept_ids)
            .await?;
        Ok(())
    }

    /// 构建登录用户信息（跨聚合：查询角色/部门/权限）
    pub async fn build_login_user(&self, user: &User) -> AppResult<LoginUser> {
        let role_ids = self.user_service.user_repo().get_role_ids(user.id).await?;
        let dept_ids = self.user_service.user_repo().get_dept_ids(user.id).await?;
        let permissions = self
            .menu_repo
            .find_permission_codes_by_user_id(user.id)
            .await?;

        Ok(LoginUser {
            user_id: user.id,
            username: user.username.clone(),
            nickname: user.nickname.clone(),
            tenant_id: user.tenant_id,
            role_ids,
            permissions,
            dept_ids,
        })
    }

    /// 根据ID获取用户信息
    pub async fn get_user(&self, user_id: u64) -> AppResult<UserResponse> {
        let user = self.user_service.get_user(user_id).await?;
        Ok(user_to_response(user))
    }

    /// 分页查询用户列表
    pub async fn get_user_page(&self, req: ListUsersRequest) -> AppResult<Page<UserResponse>> {
        let query = UserQuery {
            username: req.username,
            nickname: req.nickname,
            mobile: req.mobile,
            status: req.status.map(UserStatus::from),
            dept_id: req.dept_id,
            begin_time: None,
            end_time: None,
        };
        let pi = req.page_info.unwrap_or_default();
        let page = Page::request(pi.page, pi.size);
        let result = self.user_service.get_user_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(user_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }
}
