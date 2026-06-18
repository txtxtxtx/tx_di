use std::sync::Arc;

use crate::user::dto::*;
use admin_domain::user::model::value_object::{Sex, UserQuery, UserStatus};
use admin_domain::user::service::UserService;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::page::Page;

/// User application service - orchestrates domain operations
#[tx_comp]
pub struct UserAppService {
    user_service: Arc<UserService>,
}

impl UserAppService {
    /// 创建用户应用服务实例
    ///
    /// # 参数
    /// * `user_service` - 用户领域服务，用于执行用户相关的业务逻辑
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    /// 创建新用户
    ///
    /// # 参数
    /// * `cmd` - 创建用户命令，包含用户名、密码、昵称、邮箱、手机号、性别、备注、角色ID列表、部门ID列表
    /// * `creator` - 创建者标识（可选）
    ///
    /// # 执行逻辑
    /// 1. 校验邮箱唯一性，若已存在则返回 `DuplicateEmail` 错误
    /// 2. 校验手机号唯一性，若已存在则返回 `DuplicateMobile` 错误
    /// 3. 调用用户服务创建用户（含密码哈希处理）
    /// 4. 若提供了可选字段（邮箱、手机号、性别、备注），则更新用户信息并持久化
    /// 5. 若提供了角色ID列表，则为用户分配角色
    /// 6. 若提供了部门ID列表，则为用户分配部门
    ///
    /// # 返回
    /// 成功返回 `UserResponse`，包含完整的用户信息
    ///
    /// # 错误
    /// - `DuplicateEmail` - 邮箱已被其他用户使用
    /// - `DuplicateMobile` - 手机号已被其他用户使用
    /// - 数据库写入异常
    pub async fn create_user(
        &self,
        cmd: CreateUserCommand,
        creator: Option<String>,
    ) -> AppResult<UserResponse> {
        let email = cmd.email.filter(|s| !s.is_empty());
        let mobile = cmd.mobile.filter(|s| !s.is_empty());
        let remark = cmd.remark.filter(|s| !s.is_empty());
        let sex = cmd.sex.unwrap_or_default();

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
            .create_user(cmd.username, cmd.password, cmd.nickname, creator.clone())
            .await?;

        // Set optional fields and persist to repository
        if email.is_some() || mobile.is_some() || cmd.sex.is_some() || remark.is_some() {
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
        if let Some(role_ids) = cmd.role_ids {
            if !role_ids.is_empty() {
                self.user_service.assign_roles(user.id, role_ids.clone()).await?;
                user.role_ids = role_ids;
            }
        }

        // Assign departments if provided
        if let Some(dept_ids) = cmd.dept_ids {
            if !dept_ids.is_empty() {
                self.user_service.assign_departments(user.id, dept_ids.clone()).await?;
                user.dept_ids = dept_ids;
            }
        }

        Ok(user_to_response(user))
    }

    /// 更新用户信息
    ///
    /// # 参数
    /// * `cmd` - 更新用户命令，包含用户ID、昵称、邮箱、手机号、性别、备注
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 返回
    /// 成功返回更新后的 `UserResponse`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 数据库更新异常
    pub async fn update_user(
        &self,
        cmd: UpdateUserCommand,
        updater: Option<String>,
    ) -> AppResult<UserResponse> {
        let user = self
            .user_service
            .update_user(
                cmd.user_id,
                cmd.nickname.unwrap_or_default(),
                cmd.email.filter(|s| !s.is_empty()),
                cmd.mobile.filter(|s| !s.is_empty()),
                cmd.sex.unwrap_or_default(),
                cmd.remark.filter(|s| !s.is_empty()),
                updater,
            )
            .await?;
        Ok(user_to_response(user))
    }

    /// 删除用户
    ///
    /// # 参数
    /// * `user_id` - 要删除的用户ID
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 数据库删除异常
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service.delete_user(user_id, updater).await
    }

    /// 变更用户状态（启用/禁用）
    ///
    /// # 参数
    /// * `user_id` - 目标用户ID
    /// * `status` - 目标状态（`UserStatus` 枚举值）
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 返回
    /// 成功返回变更后的 `UserResponse`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 数据库更新异常
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
    ///
    /// # 参数
    /// * `cmd` - 修改密码命令，包含用户ID和新密码
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 密码哈希计算异常
    pub async fn change_password(
        &self,
        cmd: ChangePasswordCommand,
        updater: Option<String>,
    ) -> AppResult<()> {
        self.user_service
            .change_password(cmd.user_id, cmd.new_password, updater)
            .await?;
        Ok(())
    }

    /// 为用户分配角色
    ///
    /// # 参数
    /// * `cmd` - 分配角色命令，包含用户ID和角色ID列表
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 角色ID不存在
    pub async fn assign_roles(&self, cmd: AssignRolesCommand) -> AppResult<()> {
        self.user_service.assign_roles(cmd.user_id, cmd.role_ids).await
    }

    /// 为用户分配部门
    ///
    /// # 参数
    /// * `cmd` - 分配部门命令，包含用户ID和部门ID列表
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 部门ID不存在
    pub async fn assign_departments(&self, cmd: AssignDeptsCommand) -> AppResult<()> {
        self.user_service.assign_departments(cmd.user_id, cmd.dept_ids).await
    }

    /// 根据ID获取用户信息
    ///
    /// # 参数
    /// * `user_id` - 用户ID
    ///
    /// # 返回
    /// 成功返回 `UserResponse`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    pub async fn get_user(&self, user_id: u64) -> AppResult<UserResponse> {
        let user = self.user_service.get_user(user_id).await?;
        Ok(user_to_response(user))
    }

    /// 分页查询用户列表
    ///
    /// # 参数
    /// * `req` - 分页查询请求，包含用户名、昵称、手机号、状态、部门ID等筛选条件，以及页码和每页大小
    ///
    /// # 返回
    /// 成功返回 `Page<UserResponse>`，包含用户列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
    pub async fn get_user_page(
        &self,
        req: UserQueryRequest,
    ) -> AppResult<Page<UserResponse>> {
        let query = UserQuery {
            username: req.username,
            nickname: req.nickname,
            mobile: req.mobile,
            status: req.status,
            dept_id: req.dept_id,
            begin_time: None,
            end_time: None,
        };
        let page = Page::request(req.page, req.size);
        let result = self.user_service.get_user_page(&query, page).await?;

        Ok(Page::new(
            result.list.into_iter().map(user_to_response).collect(),
            result.page,
            result.size,
            result.total,
        ))
    }
}
