//! 文件聚合

use toasty::Model;

/// 文件实体
#[derive(Debug, Clone, Model)]
#[table = "infra_file"]
pub struct File {
    #[key]
    #[auto]
    pub id: u64,
    pub config_id: Option<u64>,
    pub name: Option<String>,
    #[column("path")]
    pub file_path: String,
    pub url: String,
    pub file_type: Option<String>,
    #[default(0i32)]
    pub size: i32,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}
