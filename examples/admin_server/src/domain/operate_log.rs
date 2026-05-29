//! 操作日志聚合

use toasty::Model;

/// 操作日志实体
#[derive(Debug, Clone, Model)]
#[table = "system_operate_log"]
pub struct OperateLog {
    #[key]
    #[auto]
    pub id: u64,
    pub trace_id: Option<String>,
    pub user_id: u64,
    #[default(0u8)]
    pub user_type: u8,
    /// 操作模块类型
    pub op_type: String,
    /// 操作名
    pub sub_type: String,
    /// 操作数据模块编号
    pub biz_id: u64,
    /// 操作内容
    #[default("".to_string())]
    pub action: String,
    /// 操作结果
    #[default(true)]
    pub success: bool,
    /// 拓展字段
    #[default("".to_string())]
    pub extra: String,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub user_ip: Option<String>,
    pub user_agent: Option<String>,
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

#[async_trait::async_trait]
pub trait OperateLogRepository: Send + Sync {
    async fn save(&self, log: &OperateLog) -> Result<(), anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, page: u64, page_size: u64) -> Result<(Vec<OperateLog>, u64), anyhow::Error>;
}
