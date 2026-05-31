//! 登录日志聚合

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum LoginLogType {
    #[column(variant = 0)] Login,
    #[column(variant = 1)] Logout,
    #[column(variant = 2)] ForceLogout,
}
impl std::fmt::Display for LoginLogType { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { LoginLogType::Login => write!(f, "login"), LoginLogType::Logout => write!(f, "logout"), LoginLogType::ForceLogout => write!(f, "force_logout") } } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum LoginResult {
    #[column(variant = 0)] Success,
    #[column(variant = 1)] BadCredentials,
    #[column(variant = 2)] UserDisabled,
    #[column(variant = 3)] CaptchaMissing,
    #[column(variant = 4)] CaptchaWrong,
    #[column(variant = 5)] Unknown,
}
impl std::fmt::Display for LoginResult { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { LoginResult::Success => write!(f, "success"), LoginResult::BadCredentials => write!(f, "bad_credentials"), LoginResult::UserDisabled => write!(f, "user_disabled"), LoginResult::CaptchaMissing => write!(f, "captcha_missing"), LoginResult::CaptchaWrong => write!(f, "captcha_wrong"), LoginResult::Unknown => write!(f, "unknown") } } }

#[derive(Debug, Clone)]
pub struct LoginLog { pub id: u64, pub log_type: LoginLogType, pub trace_id: Option<String>, pub user_id: Option<u64>, pub user_type: u8, pub username: Option<String>, pub result: LoginResult, pub user_ip: Option<String>, pub user_agent: Option<String>, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
pub trait LoginLogRepository: Send + Sync {
    async fn save(&self, log: &LoginLog) -> Result<(), anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<LoginLog>, u64), anyhow::Error>;
}
pub mod repo;
