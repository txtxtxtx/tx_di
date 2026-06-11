use async_trait::async_trait;
use tx_common::page::Page;
use tx_error::AppResult;
use crate::user::model::aggregate::User;
use crate::user::model::value_object::UserQuery;

/// User repository trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Find user by ID
    async fn find_by_id(&self, id: u64) -> AppResult<Option<User>>;

    /// Find user by username
    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>>;

    /// Find users with pagination
    async fn find_page(
        &self,
        query: &UserQuery,
        page: Page<User>,
    ) -> AppResult<Page<User>>;

    /// Find all users
    async fn find_all(&self, query: &UserQuery) -> AppResult<Vec<User>>;

    /// Insert a new user
    async fn insert(&self, user: &User) -> AppResult<()>;

    /// Update user
    async fn update(&self, user: &User) -> AppResult<()>;

    /// Soft delete user
    async fn soft_delete(&self, id: u64) -> AppResult<()>;

    /// Check if username exists
    async fn exists_by_username(&self, username: &str) -> AppResult<bool>;

    /// Check if email exists
    async fn exists_by_email(&self, email: &str) -> AppResult<bool>;

    /// Check if mobile exists
    async fn exists_by_mobile(&self, mobile: &str) -> AppResult<bool>;

    /// Count users
    async fn count(&self, query: &UserQuery) -> AppResult<i64>;

    /// Find users by role ID
    async fn find_by_role_id(&self, role_id: u64) -> AppResult<Vec<User>>;

    /// Find users by department ID
    async fn find_by_dept_id(&self, dept_id: u64) -> AppResult<Vec<User>>;

    /// Bind user to roles
    async fn bind_roles(&self, user_id: u64, role_ids: &[u64]) -> AppResult<()>;

    /// Bind user to departments
    async fn bind_departments(&self, user_id: u64, dept_ids: &[u64]) -> AppResult<()>;

    /// Get role IDs for user
    async fn get_role_ids(&self, user_id: u64) -> AppResult<Vec<u64>>;

    /// Get department IDs for user
    async fn get_dept_ids(&self, user_id: u64) -> AppResult<Vec<u64>>;
}
