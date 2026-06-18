use std::sync::Arc;

use crate::user::dto::*;
use crate::empty_string::opt_filter;
use admin_proto::{
    CreateUserRequest, UpdateUserRequest, ChangePasswordRequest,
    AssignRolesRequest, AssignDeptsRequest, ListUsersRequest,
};
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
    /// * `req` - 创建用户请求（proto），包含用户名、密码、昵称、邮箱、手机号、性别、备注、角色ID列表、部门ID列表
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
        req: CreateUserRequest,
        creator: Option<String>,
    ) -> AppResult<UserResponse> {
        let email = opt_filter(req.email);
        let mobile = opt_filter(req.mobile);
        let remark = opt_filter(req.remark);
        let sex: Sex = req.sex.map(Sex::from).unwrap_or_default();

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

        // Set optional fields and persist to repository
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
            self.user_service.assign_roles(user.id, req.role_ids.clone()).await?;
            user.role_ids = req.role_ids;
        }

        // Assign departments if provided
        if !req.dept_ids.is_empty() {
            self.user_service.assign_departments(user.id, req.dept_ids.clone()).await?;
            user.dept_ids = req.dept_ids;
        }

        Ok(user_to_response(user))
    }

    /// 更新用户信息
    ///
    /// # 参数
    /// * `req` - 更新用户请求（proto），包含用户ID、昵称、邮箱、手机号、性别、备注
    /// * `updater` - 更新者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给用户领域服务执行更新操作，逻辑详见 `UserService::update_user`
    ///
    /// # 返回
    /// 成功返回更新后的 `UserResponse`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 数据库更新异常
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
                opt_filter(req.email),
                opt_filter(req.mobile),
                req.sex.map(Sex::from).unwrap_or_default(),
                opt_filter(req.remark),
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
    /// # 执行逻辑
    /// 委托给用户领域服务执行删除操作，逻辑详见 `UserService::delete_user`
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
    /// # 执行逻辑
    /// 委托给用户领域服务执行状态变更，逻辑详见 `UserService::change_status`
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
    /// * `req` - 修改密码请求（proto），包含用户ID和新密码
    /// * `updater` - 操作者标识（可选）
    ///
    /// # 执行逻辑
    /// 委托给用户领域服务执行密码修改，逻辑详见 `UserService::change_password`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 密码哈希计算异常
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

    /// 为用户分配角色
    ///
    /// # 参数
    /// * `req` - 分配角色请求（proto），包含用户ID和角色ID列表
    ///
    /// # 执行逻辑
    /// 委托给用户领域服务执行角色分配，逻辑详见 `UserService::assign_roles`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 角色ID不存在
    pub async fn assign_roles(&self, req: AssignRolesRequest) -> AppResult<()> {
        self.user_service.assign_roles(req.user_id, req.role_ids).await
    }

    /// 为用户分配部门
    ///
    /// # 参数
    /// * `req` - 分配部门请求（proto），包含用户ID和部门ID列表
    ///
    /// # 执行逻辑
    /// 委托给用户领域服务执行部门分配，逻辑详见 `UserService::assign_departments`
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 用户ID对应的用户不存在
    /// - 部门ID不存在
    pub async fn assign_departments(&self, req: AssignDeptsRequest) -> AppResult<()> {
        self.user_service.assign_departments(req.user_id, req.dept_ids).await
    }

    /// 根据ID获取用户信息
    ///
    /// # 参数
    /// * `user_id` - 用户ID
    ///
    /// # 执行逻辑
    /// 委托给用户领域服务查询用户，逻辑详见 `UserService::get_user`
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
    /// * `req` - 分页查询请求（proto），包含用户名、昵称、手机号、状态、部门ID等筛选条件，以及页码和每页大小
    ///
    /// # 执行逻辑
    /// 1. 将请求参数转换为领域查询对象 `UserQuery`
    /// 2. 构建分页参数 `Page`
    /// 3. 委托给用户领域服务执行分页查询
    /// 4. 将领域模型列表转换为响应DTO列表
    ///
    /// # 返回
    /// 成功返回 `Page<UserResponse>`，包含用户列表、页码、每页大小和总记录数
    ///
    /// # 错误
    /// - 数据库查询异常
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
