//! 岗位聚合
//!
//! 岗位管理，用户可关联多个岗位。

use async_trait::async_trait;
use toasty::Model;
use super::dept::CommonStatus;

/// 岗位实体
#[derive(Debug, Clone, Model)]
#[table = "system_post"]
pub struct Post {
    #[key]
    #[auto]
    pub id: u64,

    /// 岗位编码
    pub code: String,

    /// 岗位名称
    pub name: String,

    /// 显示顺序
    #[default(0i32)]
    pub sort: i32,

    /// 状态
    pub status: CommonStatus,

    /// 备注
    pub remark: Option<String>,

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

impl Post {
    pub fn new(tenant_id: u64, code: String, name: String, sort: i32) -> Self {
        Self {
            id: 0,
            code,
            name,
            sort,
            status: CommonStatus::Enable,
            remark: None,
            tenant_id,
            creator: None,
            updater: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
            deleted: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status.is_enable() && self.deleted == 0
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = 1;
    }
}

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<Post>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Post>, anyhow::Error>;
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Post>, u64), anyhow::Error>;
    async fn save(&self, post: &Post) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error>;
}
