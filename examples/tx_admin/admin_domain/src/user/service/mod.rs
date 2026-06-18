use std::sync::Arc;
use tx_common::id;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::user::model::aggregate::User;
use crate::user::model::value_object::{LoginUser, Sex, UserQuery, UserStatus};
use crate::user::repository::UserRepository;
use crate::role::repository::RoleRepository;
use crate::department::repository::DepartmentRepository;
use crate::menu::repository::MenuRepository;
use crate::password;

/// User domain service
#[tx_comp]
pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
    dept_repo: Arc<dyn DepartmentRepository>,
    menu_repo: Arc<dyn MenuRepository>,
}

impl UserService {
    /// 创建 UserService 的新实例
    ///
    /// # 参数
    /// * `user_repo` - 用户仓库，用于用户相关的数据库操作
    /// * `role_repo` - 角色仓库，用于角色状态校验
    /// * `dept_repo` - 部门仓库，用于部门状态校验
    /// * `menu_repo` - 菜单仓库，用于从菜单中提取权限码
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        role_repo: Arc<dyn RoleRepository>,
        dept_repo: Arc<dyn DepartmentRepository>,
        menu_repo: Arc<dyn MenuRepository>,
    ) -> Self {
        Self {
            user_repo,
            role_repo,
            dept_repo,
            menu_repo,
        }
    }

    /// 检查邮箱是否已被注册
    ///
    /// # 参数
    /// * `email` - 待检查的邮箱地址
    ///
    /// # 执行逻辑
    /// 1. 调用用户仓库查询该邮箱是否已存在于数据库中
    ///
    /// # 返回
    /// 邮箱已存在返回 `true`，不存在返回 `false`
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn exists_by_email(&self, email: &str) -> AppResult<bool> {
        self.user_repo.exists_by_email(email).await
    }

    /// 检查手机号是否已被注册
    ///
    /// # 参数
    /// * `mobile` - 待检查的手机号
    ///
    /// # 执行逻辑
    /// 1. 调用用户仓库查询该手机号是否已存在于数据库中
    ///
    /// # 返回
    /// 手机号已存在返回 `true`，不存在返回 `false`
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn exists_by_mobile(&self, mobile: &str) -> AppResult<bool> {
        self.user_repo.exists_by_mobile(mobile).await
    }

    /// 创建新用户
    ///
    /// # 参数
    /// * `username` - 用户名，必须全局唯一
    /// * `password` - 明文密码，将使用 Argon2id 算法进行哈希加密
    /// * `nickname` - 用户昵称
    /// * `creator` - 创建者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 检查用户名是否已存在，若存在则返回重复错误
    /// 2. 使用 Argon2id 算法对明文密码进行哈希加密
    /// 3. 生成全局唯一用户 ID
    /// 4. 构建用户聚合并持久化到数据库
    ///
    /// # 返回
    /// 成功返回新创建的 `User` 聚合根
    ///
    /// # 错误
    /// - `DuplicateUsername` - 用户名已被占用
    /// - 密码哈希处理失败时返回错误
    /// - 数据库插入失败时返回错误
    pub async fn create_user(
        &self,
        username: String,
        password: String,
        nickname: String,
        creator: Option<String>,
    ) -> AppResult<User> {
        // Check if username already exists
        if self.user_repo.exists_by_username(&username).await? {
            return Err(RepositoryError::DuplicateUsername)?;
        }

        // Hash password with Argon2id
        let hashed_password = password::hash_password(&password)?;

        let user_id = id::next_id();
        let user = User::create(user_id, username, hashed_password, nickname, creator);
        self.user_repo.insert(&user).await?;
        Ok(user)
    }

    /// 更新用户基本信息
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `nickname` - 新昵称
    /// * `email` - 新邮箱，可选
    /// * `mobile` - 新手机号，可选
    /// * `sex` - 性别
    /// * `remark` - 备注，可选
    /// * `updater` - 更新者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 调用用户聚合根的 `set_basic_info` 方法更新基本信息字段
    /// 3. 将更新后的用户持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `User` 聚合根
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - 数据库更新失败时返回错误
    pub async fn update_user(
        &self,
        user_id: u64,
        nickname: String,
        email: Option<String>,
        mobile: Option<String>,
        sex: Sex,
        remark: Option<String>,
        updater: Option<String>,
    ) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        user.set_basic_info(nickname, email, mobile, sex, remark, updater);
        self.user_repo.update(&user).await?;
        Ok(user)
    }

    /// 软删除用户
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `updater` - 操作者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 调用用户聚合根的 `soft_delete` 方法标记为已删除状态
    /// 3. 将更新后的用户持久化到数据库
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - 数据库更新失败时返回错误
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        user.soft_delete(updater);
        self.user_repo.update(&user).await?;
        Ok(())
    }

    /// 变更用户状态（启用/禁用）
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `status` - 目标状态（`UserStatus::Active` 或 `UserStatus::Disabled`）
    /// * `updater` - 操作者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 调用用户聚合根的 `change_status` 方法变更状态
    /// 3. 将更新后的用户持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `User` 聚合根
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - 数据库更新失败时返回错误
    pub async fn change_status(
        &self,
        user_id: u64,
        status: UserStatus,
        updater: Option<String>,
    ) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        user.change_status(status, updater);
        self.user_repo.update(&user).await?;
        Ok(user)
    }

    /// 修改用户密码
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `password` - 新密码明文，将使用 Argon2id 算法进行哈希加密
    /// * `updater` - 操作者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 使用 Argon2id 算法对新密码进行哈希加密
    /// 3. 调用用户聚合根的 `change_password` 方法更新密码哈希值
    /// 4. 将更新后的用户持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `User` 聚合根
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - 密码哈希处理失败时返回错误
    /// - 数据库更新失败时返回错误
    pub async fn change_password(
        &self,
        user_id: u64,
        password: String,
        updater: Option<String>,
    ) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        // Hash new password with Argon2id
        let hashed_password = password::hash_password(&password)?;

        user.change_password(hashed_password, updater);
        self.user_repo.update(&user).await?;
        Ok(user)
    }

    /// 为用户分配角色
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `role_ids` - 要分配的角色 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 校验用户状态必须为 Active，否则返回状态校验错误
    /// 3. 批量查询角色列表，逐个校验每个角色必须存在且为启用状态（status == 0）
    /// 4. 调用用户仓库绑定用户与角色的关联关系
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - `ValidationUserStatus` - 用户状态非 Active 或角色状态非启用
    /// - 数据库操作失败时返回错误
    pub async fn assign_roles(
        &self,
        user_id: u64,
        role_ids: Vec<u64>,
    ) -> AppResult<()> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        // 用户必须为 Active 状态才能分配角色
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

        self.user_repo.bind_roles(user_id, &role_ids).await?;
        Ok(())
    }

    /// 为用户分配部门
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `dept_ids` - 要分配的部门 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 校验用户状态必须为 Active，否则返回状态校验错误
    /// 3. 批量查询部门列表，逐个校验每个部门必须存在且为启用状态（status == 0）
    /// 4. 调用用户仓库绑定用户与部门的关联关系
    ///
    /// # 返回
    /// 成功返回 `()`
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - `ValidationUserStatus` - 用户状态非 Active 或部门状态非启用
    /// - 数据库操作失败时返回错误
    pub async fn assign_departments(
        &self,
        user_id: u64,
        dept_ids: Vec<u64>,
    ) -> AppResult<()> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        // 用户必须为 Active 状态才能分配部门
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

        self.user_repo.bind_departments(user_id, &dept_ids).await?;
        Ok(())
    }

    /// 分页查询用户列表
    ///
    /// # 参数
    /// * `query` - 查询条件，包含用户名、状态等筛选项
    /// * `page` - 分页参数，包含页码和每页数量
    ///
    /// # 执行逻辑
    /// 1. 调用用户仓库的分页查询方法，根据条件筛选并返回分页结果
    ///
    /// # 返回
    /// 成功返回分页结果 `Page<User>`，包含用户列表和分页元数据
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn get_user_page(
        &self,
        query: &UserQuery,
        page: Page<User>,
    ) -> AppResult<Page<User>> {
        self.user_repo.find_page(query, page).await
    }

    /// 根据 ID 获取用户详情（包含角色和部门关联信息）
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 查询用户关联的角色 ID 列表并填充到用户对象
    /// 3. 查询用户关联的部门 ID 列表并填充到用户对象
    ///
    /// # 返回
    /// 成功返回包含角色和部门关联信息的 `User` 聚合根
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - 数据库查询异常时返回错误
    pub async fn get_user(&self, user_id: u64) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;
        user.role_ids = self.user_repo.get_role_ids(user.id).await?;
        user.dept_ids = self.user_repo.get_dept_ids(user.id).await?;
        Ok(user)
    }

    /// 构建登录用户信息（用于身份认证）
    ///
    /// # 参数
    /// * `user` - 用户聚合根引用
    ///
    /// # 执行逻辑
    /// 1. 查询用户关联的角色 ID 列表
    /// 2. 查询用户关联的部门 ID 列表
    /// 3. 查询用户拥有的权限列表
    /// 4. 组装 `LoginUser` 对象，包含用户 ID、用户名、昵称、租户 ID、角色、权限和部门信息
    ///
    /// # 返回
    /// 成功返回 `LoginUser` 对象，用于登录认证和权限校验
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn build_login_user(&self, user: &User) -> AppResult<LoginUser> {
        let role_ids = self.user_repo.get_role_ids(user.id).await?;
        let dept_ids = self.user_repo.get_dept_ids(user.id).await?;
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

    /// 根据用户名查询用户（用于登录验证）
    ///
    /// # 参数
    /// * `username` - 用户名
    ///
    /// # 执行逻辑
    /// 1. 调用用户仓库根据用户名查询用户记录
    ///
    /// # 返回
    /// 成功返回 `Option<User>`，用户存在返回 `Some(User)`，不存在返回 `None`
    ///
    /// # 错误
    /// - 数据库查询异常时返回错误
    pub async fn get_by_username(&self, username: &str) -> AppResult<Option<User>> {
        self.user_repo.find_by_username(username).await
    }

    /// 记录用户登录信息
    ///
    /// # 参数
    /// * `user_id` - 用户 ID
    /// * `ip` - 登录 IP 地址
    ///
    /// # 执行逻辑
    /// 1. 根据用户 ID 查询用户，若不存在则返回未找到错误
    /// 2. 调用用户聚合根的 `record_login` 方法记录登录时间和 IP 地址
    /// 3. 将更新后的用户持久化到数据库
    ///
    /// # 返回
    /// 成功返回更新后的 `User` 聚合根
    ///
    /// # 错误
    /// - `NotFoundUser` - 指定用户不存在
    /// - 数据库更新失败时返回错误
    pub async fn record_login(
        &self,
        user_id: u64,
        ip: String,
    ) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundUser)?;

        user.record_login(ip);
        self.user_repo.update(&user).await?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests;
