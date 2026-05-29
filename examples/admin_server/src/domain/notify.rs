//! 站内信聚合

use toasty::Model;

/// 站内信模板实体
#[derive(Debug, Clone, Model)]
#[table = "system_notify_template"]
pub struct NotifyTemplate {
    #[key]
    #[auto]
    pub id: u64,
    pub name: String,
    #[unique]
    pub code: String,
    pub nickname: String,
    pub content: String,
    /// 模板类型
    #[default(0u16)]
    pub template_type: u16,
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

/// 站内信消息实体
#[derive(Debug, Clone, Model)]
#[table = "system_notify_message"]
pub struct NotifyMessage {
    #[key]
    #[auto]
    pub id: u64,
    /// 接收用户 ID
    pub user_id: u64,
    /// 用户类型
    #[default(0u8)]
    pub user_type: u8,
    /// 模板 ID
    pub template_id: u64,
    /// 模板编码
    pub template_code: String,
    /// 发送人名称
    pub template_nickname: String,
    /// 消息内容
    pub template_content: String,
    /// 模板类型
    #[default(0u16)]
    pub template_type: u16,
    /// 模板参数
    pub template_params: String,
    /// 是否已读
    #[default(false)]
    pub read_status: bool,
    /// 阅读时间
    pub read_time: Option<jiff::Timestamp>,
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

impl NotifyMessage {
    /// 标记为已读
    pub fn mark_read(&mut self) {
        self.read_status = true;
        self.read_time = Some(jiff::Timestamp::now());
    }

    /// 是否已读
    pub fn is_read(&self) -> bool {
        self.read_status
    }
}

#[async_trait::async_trait]
pub trait NotifyRepository: Send + Sync {
    async fn find_template_by_id(&self, id: u64) -> Result<Option<NotifyTemplate>, anyhow::Error>;
    async fn find_template_page(&self, page: u64, page_size: u64) -> Result<(Vec<NotifyTemplate>, u64), anyhow::Error>;
    async fn save_template(&self, tpl: &NotifyTemplate) -> Result<(), anyhow::Error>;
    async fn delete_template(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn save_message(&self, msg: &NotifyMessage) -> Result<(), anyhow::Error>;
    async fn find_message_page(&self, user_id: u64, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<NotifyMessage>, u64), anyhow::Error>;
    async fn count_unread(&self, user_id: u64, tenant_id: u64) -> Result<u64, anyhow::Error>;
    async fn mark_read(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn mark_all_read(&self, user_id: u64, tenant_id: u64) -> Result<(), anyhow::Error>;
}
