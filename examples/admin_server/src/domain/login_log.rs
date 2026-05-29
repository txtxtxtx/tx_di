//! 登录日志聚合
//!
//! 记录用户的登录/登出行为，支持登录结果分析和异常检测。

use serde::{Deserialize, Serialize};
use toasty::Model;

/// 登录日志类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum LoginLogType {
    /// 登录
    #[column(variant = 100)]
    Login,
    /// 登出
    #[column(variant = 200)]
    Logout,
    /// 强制登出
    #[column(variant = 202)]
    ForceLogout,
}

/// 登录结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum LoginResult {
    /// 成功
    #[column(variant = 0)]
    Success,
    /// 账号或密码不正确
    #[column(variant = 10)]
    BadCredentials,
    /// 用户被禁用
    #[column(variant = 20)]
    UserDisabled,
    /// 验证码不存在
    #[column(variant = 30)]
    CaptchaMissing,
    /// 验证码不正确
    #[column(variant = 31)]
    CaptchaWrong,
    /// 未知异常
    #[column(variant = 100)]
    Unknown,
}

/// 登录日志实体
#[derive(Debug, Clone, Model)]
#[table = "system_login_log"]
pub struct LoginLog {
    #[key]
    #[auto]
    pub id: u64,

    /// 日志类型
    pub log_type: LoginLogType,

    /// 链路追踪 ID
    pub trace_id: Option<String>,

    /// 用户 ID
    pub user_id: Option<u64>,

    /// 用户类型（0=管理员, 1=会员）
    #[default(0u8)]
    pub user_type: u8,

    /// 用户名
    pub username: Option<String>,

    /// 登录结果
    pub result: LoginResult,

    /// 用户 IP
    pub user_ip: Option<String>,

    /// User Agent
    pub user_agent: Option<String>,

    /// 所属租户 ID
    pub tenant_id: u64,

    pub creator: Option<String>,
    pub updater: Option<String>,

    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,

    #[default(0u8)]
    pub deleted: u8,
}

impl LoginLog {
    /// 创建登录成功日志
    pub fn login_success(
        user_id: u64,
        username: String,
        tenant_id: u64,
        ip: String,
        user_agent: String,
    ) -> Self {
        Self::new(LoginLogType::Login, LoginResult::Success, Some(user_id), Some(username), tenant_id, Some(ip), Some(user_agent))
    }

    /// 创建登录失败日志
    pub fn login_failure(
        username: String,
        tenant_id: u64,
        result: LoginResult,
        ip: String,
        user_agent: String,
    ) -> Self {
        Self::new(LoginLogType::Login, result, None, Some(username), tenant_id, Some(ip), Some(user_agent))
    }

    fn new(
        log_type: LoginLogType,
        result: LoginResult,
        user_id: Option<u64>,
        username: Option<String>,
        tenant_id: u64,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            id: 0, log_type, trace_id: None, user_id, user_type: 0,
            username, result, user_ip: ip, user_agent, tenant_id,
            creator: None, updater: None,
            created_at: jiff::Timestamp::now(), updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    /// 是否登录成功
    pub fn is_success(&self) -> bool {
        self.result == LoginResult::Success
    }
}

/// 登录日志仓储 trait
#[async_trait::async_trait]
pub trait LoginLogRepository: Send + Sync {
    async fn save(&self, log: &LoginLog) -> Result<(), anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<LoginLog>, u64), anyhow::Error>;
}
