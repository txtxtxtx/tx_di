//! OAuth2 聚合

use toasty::Model;

/// OAuth2 客户端实体
#[derive(Debug, Clone, Model)]
#[table = "system_oauth2_client"]
pub struct OAuth2Client {
    #[key]
    #[auto]
    pub id: u64,
    #[unique]
    pub client_id: String,
    pub secret: String,
    pub name: Option<String>,
    pub logo: Option<String>,
    pub description: Option<String>,
    #[default(0u8)]
    pub status: u8,
    #[default(7200i32)]
    pub access_token_validity_seconds: i32,
    #[default(2592000i32)]
    pub refresh_token_validity_seconds: i32,
    pub redirect_uris: Option<String>,
    pub authorized_grant_types: Option<String>,
    pub scopes: Option<String>,
    pub auto_approve_scopes: Option<String>,
    pub authorities: Option<String>,
    pub resource_ids: Option<String>,
    pub additional_information: Option<String>,
    pub creator: Option<String>,
    pub updater: Option<String>,
    #[auto]
    pub created_at: jiff::Timestamp,
    #[default(jiff::Timestamp::now())]
    pub updated_at: jiff::Timestamp,
    #[default(0u8)]
    pub deleted: u8,
}

/// OAuth2 访问令牌实体
#[derive(Debug, Clone, Model)]
#[table = "system_oauth2_access_token"]
pub struct OAuth2AccessToken {
    #[key]
    #[auto]
    pub id: u64,
    pub user_id: u64,
    #[default(0u8)]
    pub user_type: u8,
    pub user_info: Option<String>,
    #[unique]
    pub access_token: String,
    #[index]
    pub refresh_token: Option<String>,
    pub client_id: String,
    pub scopes: Option<String>,
    pub expires_time: jiff::Timestamp,
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

/// OAuth2 刷新令牌实体
#[derive(Debug, Clone, Model)]
#[table = "system_oauth2_refresh_token"]
pub struct OAuth2RefreshToken {
    #[key]
    #[auto]
    pub id: u64,
    pub user_id: u64,
    #[default(0u8)]
    pub user_type: u8,
    #[unique]
    pub refresh_token: String,
    pub access_token_id: Option<u64>,
    pub client_id: String,
    pub scopes: Option<String>,
    pub expires_time: jiff::Timestamp,
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
pub trait OAuth2Repository: Send + Sync {
    async fn find_client_by_id(&self, id: u64) -> Result<Option<OAuth2Client>, anyhow::Error>;
    async fn find_client_by_client_id(&self, client_id: &str) -> Result<Option<OAuth2Client>, anyhow::Error>;
    async fn find_all_clients(&self) -> Result<Vec<OAuth2Client>, anyhow::Error>;
    async fn save_client(&self, client: &OAuth2Client) -> Result<(), anyhow::Error>;
    async fn delete_client(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_access_token(&self, token: &str) -> Result<Option<OAuth2AccessToken>, anyhow::Error>;
    async fn save_access_token(&self, token: &OAuth2AccessToken) -> Result<(), anyhow::Error>;
    async fn delete_access_token(&self, id: u64) -> Result<(), anyhow::Error>;
    async fn find_refresh_token(&self, token: &str) -> Result<Option<OAuth2RefreshToken>, anyhow::Error>;
    async fn save_refresh_token(&self, token: &OAuth2RefreshToken) -> Result<(), anyhow::Error>;
    async fn delete_refresh_token(&self, id: u64) -> Result<(), anyhow::Error>;
}
