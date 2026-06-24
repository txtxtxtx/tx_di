//! 认证会话服务
//!
//! 封装所有 Session/Token 操作（sa-token），
//! 使 API 层不再直接依赖 `StpUtil`。

use tx_di_core::tx_comp;
use tx_di_sa_token::StpUtil;
use tx_error::{AppError, AppResult};
use admin_domain::shared::model::value_object::SessionEctData;

/// 认证会话服务
///
/// 负责：
/// - 登录时创建 sa-token 会话，绑定角色/权限/extra_data
/// - 登出时销毁会话、清除权限/角色缓存
#[tx_comp]
pub struct AuthSessionService;

impl AuthSessionService {
    pub fn new() -> Self {
        Self
    }

    /// 用户登录：创建 sa-token 会话并返回 token 字符串
    ///
    /// # 参数
    /// - `user_id` - 用户 ID
    /// - `is_admin` - 是否为管理员（管理员跳过权限绑定）
    ///
    /// # 执行逻辑
    /// 1. 调用 StpUtil::login 创建会话获取 token
    /// 2. 将 session 扩展数据（登录IP/租户/角色/部门/用户名）写入 extra_data
    /// 3. 非管理员：绑定权限码集合
    /// 4. 绑定角色编码集合
    ///
    /// # 返回
    /// 成功返回 token 字符串
    pub async fn login(
        &self,
        user_id: u64,
        is_admin: bool,
        extra: SessionEctData,
        permissions: Vec<String>,
        role_codes: Vec<String>,
    ) -> AppResult<String> {
        let login_id = user_id.to_string();
        let token = StpUtil::login(&login_id).await.map_err(|e| AppError::from(e.to_string()))?;
        let token_str = token.to_string();

        // 写入扩展数据（供在线用户查询、操作日志中间件等使用）
        let extra_json = serde_json::json!(extra);
        let _ = StpUtil::set_extra_data(&token, extra_json).await;

        // 非管理员绑定具体权限码
        if !is_admin {
            StpUtil::set_permissions(&login_id, permissions).await.map_err(|e| AppError::from(e.to_string()))?;
        }
        // 绑定角色编码
        StpUtil::set_roles(&login_id, role_codes).await.map_err(|e| AppError::from(e.to_string()))?;

        Ok(token_str)
    }

    /// 用户登出：销毁当前 sa-token 会话
    ///
    /// # 参数
    /// - `login_id` - 登录 ID（user_id 的字符串形式）
    ///
    /// # 执行逻辑
    /// 1. 使当前 sa-token 会话失效
    /// 2. 清除该用户的权限缓存
    /// 3. 清除该用户的角色缓存
    pub async fn logout(&self, login_id: &str) -> AppResult<()> {
        StpUtil::logout_current().await.map_err(|e| AppError::from(e.to_string()))?;
        StpUtil::clear_permissions(login_id).await.map_err(|e| AppError::from(e.to_string()))?;
        StpUtil::clear_roles(login_id).await.map_err(|e| AppError::from(e.to_string()))?;
        Ok(())
    }
}
