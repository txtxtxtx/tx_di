//! 认证领域服务
//!
//! 封装登录认证的核心业务逻辑：
//! - 验证用户名密码
//! - 检查账号状态
//!
//! 本服务通过 `UserService` 获取用户数据，不直接依赖 Repository。

use std::sync::Arc;

use tx_di_core::{Component, DepsTuple};
use tx_error::{AppError, AppResult};

use crate::user::model::aggregate::User;
use crate::user::service::UserService;
use crate::auth::error::AuthError;

/// 认证领域服务
///
/// 提供 `authenticate` 方法，统一处理登录认证逻辑，
/// 返回明确的 `AuthError` 领域错误。
#[derive(Component)]
pub struct AuthService {
    user_service: Arc<UserService>,
}

impl AuthService {
    /// 认证用户登录
    ///
    /// # 流程
    /// 1. 按用户名查找用户
    /// 2. 检查用户是否激活
    /// 3. 验证密码
    ///
    /// # 错误
    /// - `AuthError::UserNotFound` — 用户名不存在
    /// - `AuthError::UserDisabled` — 用户被禁用或已删除
    /// - `AuthError::InvalidCredentials` — 密码错误
    pub async fn authenticate(&self, username: &str, password: &str) -> AppResult<User> {
        let user = self
            .user_service
            .get_by_username(username)
            .await?
            .ok_or_else(|| AppError::from_code(AuthError::UserNotFound))?;

        if !user.is_active() {
            return Err(AppError::from_code(AuthError::UserDisabled));
        }

        if !user.verify_password(password) {
            return Err(AppError::from_code(AuthError::InvalidCredentials));
        }

        Ok(user)
    }
}
