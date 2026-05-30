//! OAuth2 聚合

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct OAuth2Client { pub id: u64, pub client_id: String, pub secret: String, pub name: Option<String>, pub logo: Option<String>, pub description: Option<String>, pub status: u8, pub access_token_validity_seconds: i32, pub refresh_token_validity_seconds: i32, pub redirect_uris: Option<String>, pub authorized_grant_types: Option<String>, pub scopes: Option<String>, pub auto_approve_scopes: Option<String>, pub authorities: Option<String>, pub resource_ids: Option<String>, pub additional_information: Option<String>, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[derive(Debug, Clone)]
pub struct OAuth2AccessToken { pub id: u64, pub user_id: u64, pub user_type: u8, pub user_info: Option<String>, pub access_token: String, pub refresh_token: Option<String>, pub client_id: String, pub scopes: Option<String>, pub expires_time: jiff::Timestamp, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[derive(Debug, Clone)]
pub struct OAuth2RefreshToken { pub id: u64, pub user_id: u64, pub user_type: u8, pub refresh_token: String, pub access_token_id: Option<u64>, pub client_id: String, pub scopes: Option<String>, pub expires_time: jiff::Timestamp, pub tenant_id: u64, pub creator: Option<String>, pub updater: Option<String>, pub created_at: jiff::Timestamp, pub updated_at: jiff::Timestamp, pub deleted: u8 }

#[async_trait]
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
pub mod repo;
