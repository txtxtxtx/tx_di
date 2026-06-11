use std::sync::Arc;
use tx_common::id;
use tx_common::page::Page;
use tx_error::AppResult;
use crate::shared::repository::RepositoryError;
use crate::user::model::aggregate::User;
use crate::user::model::value_object::{LoginUser, Sex, UserQuery, UserStatus};
use crate::user::repository::UserRepository;
use crate::permission::repository::PermissionRepository;
use crate::shared::repository::RepositoryError::NotFound;

/// User domain service
pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
    permission_repo: Arc<dyn PermissionRepository>,
}

impl UserService {
    /// 创建 UserService 的新实例
    /// 
    /// # 参数
    /// * `user_repo` - 用户仓库，用于用户相关的数据库操作
    /// * `permission_repo` - 权限仓库，用于权限相关的数据库操作
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        permission_repo: Arc<dyn PermissionRepository>,
    ) -> Self {
        Self {
            user_repo,
            permission_repo,
        }
    }

    /// Check if email already exists
    pub async fn exists_by_email(&self, email: &str) -> AppResult<bool> {
        self.user_repo.exists_by_email(email).await
    }

    /// Check if mobile already exists
    pub async fn exists_by_mobile(&self, mobile: &str) -> AppResult<bool> {
        self.user_repo.exists_by_mobile(mobile).await
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        username: String,
        password: String,
        nickname: String,
        creator: Option<String>,
    ) -> AppResult<User> {
        // Check if username already exists
        if self.user_repo.exists_by_username(&username).await? {
            return Err(RepositoryError::Duplicate)?;
        }

        let user_id = id::next_id();
        let user = User::create(user_id, username, password, nickname, creator);
        self.user_repo.insert(&user).await?;
        Ok(user)
    }

    /// Update user basic info
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
            .ok_or_else(|| NotFound)?;

        user.set_basic_info(nickname, email, mobile, sex, remark, updater);
        self.user_repo.update(&user).await?;
        Ok(user)
    }

    /// Delete user
    pub async fn delete_user(
        &self,
        user_id: u64,
        updater: Option<String>,
    ) -> AppResult<()> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| NotFound)?;

        user.soft_delete(updater);
        self.user_repo.update(&user).await?;
        Ok(())
    }

    /// Change user status
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
            .ok_or_else(|| NotFound)?;

        user.change_status(status, updater);
        self.user_repo.update(&user).await?;
        Ok(user)
    }

    /// Change user password
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
            .ok_or_else(|| NotFound)?;

        user.change_password(password, updater);
        self.user_repo.update(&user).await?;
        Ok(user)
    }

    /// Assign roles to user
    pub async fn assign_roles(
        &self,
        user_id: u64,
        role_ids: Vec<u64>,
    ) -> AppResult<()> {
        // Verify user exists
        let _user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| NotFound)?;

        self.user_repo.bind_roles(user_id, &role_ids).await?;
        Ok(())
    }

    /// Assign departments to user
    pub async fn assign_departments(
        &self,
        user_id: u64,
        dept_ids: Vec<u64>,
    ) -> AppResult<()> {
        let _user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| NotFound)?;

        self.user_repo.bind_departments(user_id, &dept_ids).await?;
        Ok(())
    }

    /// Get user page
    pub async fn get_user_page(
        &self,
        query: &UserQuery,
        page: Page<User>,
    ) -> AppResult<Page<User>> {
        self.user_repo.find_page(query, page).await
    }

    /// Get user by ID (includes role and department associations)
    pub async fn get_user(&self, user_id: u64) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| NotFound)?;
        user.role_ids = self.user_repo.get_role_ids(user.id).await?;
        user.dept_ids = self.user_repo.get_dept_ids(user.id).await?;
        Ok(user)
    }

    /// Build login user info (for auth)
    pub async fn build_login_user(&self, user: &User) -> AppResult<LoginUser> {
        let role_ids = self.user_repo.get_role_ids(user.id).await?;
        let dept_ids = self.user_repo.get_dept_ids(user.id).await?;
        let permissions = self.permission_repo.find_by_user_id(user.id).await?;

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

    /// Get user by username (for login)
    pub async fn get_by_username(&self, username: &str) -> AppResult<Option<User>> {
        self.user_repo.find_by_username(username).await
    }

    /// Record login
    pub async fn record_login(
        &self,
        user_id: u64,
        ip: String,
    ) -> AppResult<User> {
        let mut user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| NotFound)?;

        user.record_login(ip);
        self.user_repo.update(&user).await?;
        Ok(user)
    }
}
