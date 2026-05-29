//! 邮件聚合

use serde::{Deserialize, Serialize};
use toasty::Model;

/// 发送状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
pub enum MailSendStatus {
    /// 初始
    #[column(variant = 0)]
    Init,
    /// 成功
    #[column(variant = 10)]
    Success,
    /// 失败
    #[column(variant = 20)]
    Failed,
}

/// 邮件账号实体
#[derive(Debug, Clone, Model)]
#[table = "system_mail_account"]
pub struct MailAccount {
    #[key]
    #[auto]
    pub id: u64,
    pub mail: String,
    pub username: String,
    pub password: String,
    pub host: String,
    #[default(25i32)]
    pub port: i32,
    #[default(false)]
    pub ssl_enable: bool,
    #[default(false)]
    pub starttls_enable: bool,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

/// 邮件模板实体
#[derive(Debug, Clone, Model)]
#[table = "system_mail_template"]
pub struct MailTemplate {
    #[key]
    #[auto]
    pub id: u64,
    pub name: String,
    #[unique]
    pub code: String,
    pub account_id: u64,
    pub nickname: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub params: Option<String>,
    #[default(0u8)]
    pub status: u8,
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

/// 邮件日志实体
#[derive(Debug, Clone, Model)]
#[table = "system_mail_log"]
pub struct MailLog {
    #[key]
    #[auto]
    pub id: u64,
    pub user_id: Option<u64>,
    #[default(0u8)]
    pub user_type: u8,
    pub to_mails: Option<String>,
    pub cc_mails: Option<String>,
    pub bcc_mails: Option<String>,
    pub account_id: Option<u64>,
    pub from_mail: Option<String>,
    pub template_id: Option<u64>,
    pub template_code: Option<String>,
    pub template_nickname: Option<String>,
    pub template_title: Option<String>,
    pub template_content: Option<String>,
    pub template_params: Option<String>,
    pub send_status: MailSendStatus,
    pub send_time: Option<jiff::Timestamp>,
    pub send_message_id: Option<String>,
    pub send_exception: Option<String>,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

#[async_trait::async_trait]
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
