//! 邮件聚合

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailSendStatus { Init, Success, Failed }
impl std::fmt::Display for MailSendStatus { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { MailSendStatus::Init => write!(f, "init"), MailSendStatus::Success => write!(f, "success"), MailSendStatus::Failed => write!(f, "failed") } } }

#[derive(Debug, Clone)]
pub struct MailAccount { pub id: u64, pub mail: String, pub username: String, pub password: String, pub host: String, pub port: i32, pub ssl_enable: bool, pub starttls_enable: bool, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[derive(Debug, Clone)]
pub struct MailTemplate { pub id: u64, pub name: String, pub code: String, pub account_id: u64, pub nickname: Option<String>, pub title: Option<String>, pub content: Option<String>, pub params: Option<String>, pub status: u8, pub remark: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[derive(Debug, Clone)]
pub struct MailLog { pub id: u64, pub user_id: Option<u64>, pub user_type: u8, pub to_mails: Option<String>, pub cc_mails: Option<String>, pub bcc_mails: Option<String>, pub account_id: Option<u64>, pub from_mail: Option<String>, pub template_id: Option<u64>, pub template_code: Option<String>, pub template_nickname: Option<String>, pub template_title: Option<String>, pub template_content: Option<String>, pub template_params: Option<String>, pub send_status: MailSendStatus, pub send_time: Option<jiff::Timestamp>, pub send_message_id: Option<String>, pub send_exception: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
pub trait MailRepository: Send + Sync {
    async fn find_account_by_id(&self, id: u64) -> Result<Option<MailAccount>, anyhow::Error>;
    async fn find_all_accounts(&self) -> Result<Vec<MailAccount>, anyhow::Error>;
    async fn save_account(&self, account: &MailAccount) -> Result<(), anyhow::Error>;
    async fn delete_account(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_template_by_id(&self, id: u64) -> Result<Option<MailTemplate>, anyhow::Error>;
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailTemplate>, u64), anyhow::Error>;
    async fn save_template(&self, tpl: &MailTemplate) -> Result<(), anyhow::Error>;
    async fn delete_template(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn save_log(&self, log: &MailLog) -> Result<(), anyhow::Error>;
    async fn find_log_page(&self, page: u64, page_size: u64) -> Result<(Vec<MailLog>, u64), anyhow::Error>;
}
pub mod repo;
