use std::sync::Arc;

use crate::user::dto::*;
use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    ListUsersRequest,
};
use admin_domain::user::model::aggregate::User;
use admin_domain::user::model::value_object::{LoginUser, Sex, UserQuery, UserStatus};
use admin_domain::user::service::UserService;
use admin_domain::role::repository::RoleRepository;
use admin_domain::department::repository::DepartmentRepository;
use admin_domain::menu::repository::MenuRepository;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;
use tx_common::page::Page;

/// User application service - 编排领域操作 + 跨聚合校验
#[derive(Component)]
pub struct UserAppService {
    user_service: Arc<UserService>,
    role_repo: Arc<dyn RoleRepository>,
    dept_repo: Arc<dyn DepartmentRepository>,
    menu_repo: Arc<dyn MenuRepository>,
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
        Self { user_service, role_repo, dept_repo, menu_repo }
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
        if let Some(ref e) = email {
            if self.user_service.exists_by_email(e).await? {
                return Err(RepositoryError::DuplicateEmail)?;
            }
        }

        // Check mobile uniqueness
        if let Some(ref m) = mobile {
            if self.user_service.exists_by_mobile(m).await? {
                return Err(RepositoryError::DuplicateMobile)?;
            }
        }

        let mut user = self
            .user_service
            .create_user(req.username, req.password, req.nickname, creator.clone())
            .await?;

        // Set optional fields and persist
        if email.is_some() || mobile.is_some() || req.sex.is_some() || remark.is_some() {
            user.email = email;
            user.mobile = mobile;
            user.sex = sex;
            user.remark = remark;
            user = self
                .user_service
                .update_user(
                    user.id,
                    user.nickname.clone(),
                    user.email.clone(),
                    user.mobile.clone(),
                    user.sex,
                    user.remark.clone(),
                    creator.clone(),
                )
                .await?;
        }

        // Assign roles if provided
        if !req.role_ids.is_empty() {
            self.assign_roles(user.id, req.role_ids.clone()).await?;
            user.role_ids = req.role_ids;
        }

        // Assign departments if provided
        if !req.dept_ids.is_empty() {
            self.assign_departments(user.id, req.dept_ids.clone()).await?;
            user.dept_ids = req.dept_ids;
        }

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
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service.delete_user(user_id, updater).await
    }

    /// 变更用户状态
    pub async fn change_status(
        &self,
        user_id: u64,
        status: UserStatus,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self.user_service.change_status(user_id, status, updater).await?;
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
        let user = self
            .user_service
            .get_user(user_id)
            .await?;

        // 用户必须为 Active 状态
        if user.status != UserStatus::Active {
            return Err(RepositoryError::ValidationUserStatus)?;
        }

        // 校验每个角色存在且为启用状态（status == 0 即 Enabled）
        let roles = self.role_repo.find_by_ids(&role_ids).await?;
        for r in &roles {
            if r.status != 0 {
                return Err(RepositoryError::ValidationUserStatus)?;
            }
        }

        self.user_service.user_repo().bind_roles(user_id, &role_ids).await?;
        Ok(())
    }

    /// 为用户分配部门（跨聚合校验：校验部门存在且启用）
    pub async fn assign_departments(&self, user_id: u64, dept_ids: Vec<u64>) -> AppResult<()> {
        let user = self
            .user_service
            .get_user(user_id)
            .await?;

        // 用户必须为 Active 状态
        if user.status != UserStatus::Active {
            return Err(RepositoryError::ValidationUserStatus)?;
        }

        // 校验每个部门存在且为启用状态
        let depts = self.dept_repo.find_by_ids(&dept_ids).await?;
        for d in &depts {
            if d.status != 0 {
                return Err(RepositoryError::ValidationDeptDisabled)?;
            }
        }

        self.user_service.user_repo().bind_departments(user_id, &dept_ids).await?;
        Ok(())
    }

    /// 构建登录用户信息（跨聚合：查询角色/部门/权限）
    pub async fn build_login_user(&self, user: &User) -> AppResult<LoginUser> {
        let role_ids = self.user_service.user_repo().get_role_ids(user.id).await?;
        let dept_ids = self.user_service.user_repo().get_dept_ids(user.id).await?;
        let permissions = self.menu_repo.find_permission_codes_by_user_id(user.id).await?;

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
    pub async fn get_user_page(
        &self,
        req: ListUsersRequest,
    ) -> AppResult<Page<UserResponse>> {
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
