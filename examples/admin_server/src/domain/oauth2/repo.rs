//! OAuth2 仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{OAuth2Client, OAuth2AccessToken, OAuth2RefreshToken, OAuth2Repository};

#[derive(Debug, Clone, Model)]
#[table = "system_oauth2_client"]
pub struct OAuth2ClientModel {
    #[key] #[auto] pub id: u64, #[unique] pub client_id: String, pub secret: String,
    #[default("".to_string())] pub name: String, #[default("".to_string())] pub logo: String, #[default("".to_string())] pub description: String,
    #[default(0u8)] pub status: u8, #[default(7200i32)] pub access_token_validity_seconds: i32,
    #[default(2592000i32)] pub refresh_token_validity_seconds: i32,
    #[default("".to_string())] pub redirect_uris: String, #[default("".to_string())] pub authorized_grant_types: String,
    #[default("".to_string())] pub scopes: String, #[default("".to_string())] pub auto_approve_scopes: String,
    #[default("".to_string())] pub authorities: String, #[default("".to_string())] pub resource_ids: String,
    #[default("".to_string())] pub additional_information: String, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_oauth2_access_token"]
pub struct OAuth2AccessTokenModel {
    #[key] #[auto] pub id: u64, #[index] pub user_id: i64, #[default(0u8)] pub user_type: u8,
    #[default("".to_string())] pub user_info: String, #[unique] pub access_token: String,
    #[index] #[default("".to_string())] pub refresh_token: String, pub client_id: String,
    #[default("".to_string())] pub scopes: String, pub expires_time: jiff::Timestamp, #[index] pub tenant_id: i64,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_oauth2_refresh_token"]
pub struct OAuth2RefreshTokenModel {
    #[key] #[auto] pub id: u64, #[index] pub user_id: i64, #[default(0u8)] pub user_type: u8,
    #[unique] pub refresh_token: String, #[default(0i64)] pub access_token_id: i64,
    pub client_id: String, #[default("".to_string())] pub scopes: String, pub expires_time: jiff::Timestamp,
    #[index] pub tenant_id: i64, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<OAuth2ClientModel> for OAuth2Client { fn from(m: OAuth2ClientModel) -> Self { Self { id: m.id, client_id: m.client_id, secret: m.secret, name: if m.name.is_empty() { None } else { Some(m.name) }, logo: if m.logo.is_empty() { None } else { Some(m.logo) }, description: if m.description.is_empty() { None } else { Some(m.description) }, status: m.status, access_token_validity_seconds: m.access_token_validity_seconds, refresh_token_validity_seconds: m.refresh_token_validity_seconds, redirect_uris: if m.redirect_uris.is_empty() { None } else { Some(m.redirect_uris) }, authorized_grant_types: if m.authorized_grant_types.is_empty() { None } else { Some(m.authorized_grant_types) }, scopes: if m.scopes.is_empty() { None } else { Some(m.scopes) }, auto_approve_scopes: if m.auto_approve_scopes.is_empty() { None } else { Some(m.auto_approve_scopes) }, authorities: if m.authorities.is_empty() { None } else { Some(m.authorities) }, resource_ids: if m.resource_ids.is_empty() { None } else { Some(m.resource_ids) }, additional_information: if m.additional_information.is_empty() { None } else { Some(m.additional_information) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<OAuth2AccessTokenModel> for OAuth2AccessToken { fn from(m: OAuth2AccessTokenModel) -> Self { Self { id: m.id, user_id: m.user_id as u64, user_type: m.user_type, user_info: if m.user_info.is_empty() { None } else { Some(m.user_info) }, access_token: m.access_token, refresh_token: if m.refresh_token.is_empty() { None } else { Some(m.refresh_token) }, client_id: m.client_id, scopes: if m.scopes.is_empty() { None } else { Some(m.scopes) }, expires_time: m.expires_time, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }
impl From<OAuth2RefreshTokenModel> for OAuth2RefreshToken { fn from(m: OAuth2RefreshTokenModel) -> Self { Self { id: m.id, user_id: m.user_id as u64, user_type: m.user_type, refresh_token: m.refresh_token, access_token_id: if m.access_token_id == 0 { None } else { Some(m.access_token_id as u64) }, client_id: m.client_id, scopes: if m.scopes.is_empty() { None } else { Some(m.scopes) }, expires_time: m.expires_time, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyOAuth2Repository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl OAuth2Repository for ToastyOAuth2Repository {
    async fn find_client_by_id(&self, id: u64) -> Result<Option<OAuth2Client>, anyhow::Error> { let mut db = self.toasty.db().clone(); match OAuth2ClientModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(OAuth2Client::from(m))), Err(_) => Ok(None) } }
    async fn find_client_by_client_id(&self, client_id: &str) -> Result<Option<OAuth2Client>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(OAuth2ClientModel::filter_by_client_id(client_id.to_string()).first().exec(&mut db).await?.map(OAuth2Client::from)) }
    async fn find_all_clients(&self) -> Result<Vec<OAuth2Client>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(OAuth2ClientModel::all().exec(&mut db).await?.into_iter().filter(|m| m.deleted == 0).map(OAuth2Client::from).collect()) }
    async fn save_client(&self, client: &OAuth2Client) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if client.id == 0 { toasty::create!(OAuth2ClientModel { client_id: client.client_id.clone(), secret: client.secret.clone(), name: client.name.clone().unwrap_or_default(), logo: client.logo.clone().unwrap_or_default(), description: client.description.clone().unwrap_or_default(), status: client.status, access_token_validity_seconds: client.access_token_validity_seconds, refresh_token_validity_seconds: client.refresh_token_validity_seconds, redirect_uris: client.redirect_uris.clone().unwrap_or_default(), authorized_grant_types: client.authorized_grant_types.clone().unwrap_or_default(), scopes: client.scopes.clone().unwrap_or_default(), auto_approve_scopes: client.auto_approve_scopes.clone().unwrap_or_default(), authorities: client.authorities.clone().unwrap_or_default(), resource_ids: client.resource_ids.clone().unwrap_or_default(), additional_information: client.additional_information.clone().unwrap_or_default(), creator: client.creator.clone().unwrap_or_default(), updater: client.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = OAuth2ClientModel::get_by_id(&mut db, client.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.client_id = client.client_id.clone(); m.secret = client.secret.clone(); m.name = client.name.clone().unwrap_or_default(); m.logo = client.logo.clone().unwrap_or_default(); m.description = client.description.clone().unwrap_or_default(); m.status = client.status; m.access_token_validity_seconds = client.access_token_validity_seconds; m.refresh_token_validity_seconds = client.refresh_token_validity_seconds; m.redirect_uris = client.redirect_uris.clone().unwrap_or_default(); m.authorized_grant_types = client.authorized_grant_types.clone().unwrap_or_default(); m.scopes = client.scopes.clone().unwrap_or_default(); m.auto_approve_scopes = client.auto_approve_scopes.clone().unwrap_or_default(); m.authorities = client.authorities.clone().unwrap_or_default(); m.resource_ids = client.resource_ids.clone().unwrap_or_default(); m.additional_information = client.additional_information.clone().unwrap_or_default(); m.creator = client.creator.clone().unwrap_or_default(); m.updater = client.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete_client(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match OAuth2ClientModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn find_access_token(&self, token: &str) -> Result<Option<OAuth2AccessToken>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(OAuth2AccessTokenModel::filter_by_access_token(token.to_string()).first().exec(&mut db).await?.map(OAuth2AccessToken::from)) }
    async fn save_access_token(&self, token: &OAuth2AccessToken) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); toasty::create!(OAuth2AccessTokenModel { user_id: token.user_id as i64, user_type: token.user_type, user_info: token.user_info.clone().unwrap_or_default(), access_token: token.access_token.clone(), refresh_token: token.refresh_token.clone().unwrap_or_default(), client_id: token.client_id.clone(), scopes: token.scopes.clone().unwrap_or_default(), expires_time: token.expires_time, tenant_id: token.tenant_id as i64, creator: token.creator.clone().unwrap_or_default(), updater: token.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(()) }
    async fn delete_access_token(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match OAuth2AccessTokenModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn find_refresh_token(&self, token: &str) -> Result<Option<OAuth2RefreshToken>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(OAuth2RefreshTokenModel::filter_by_refresh_token(token.to_string()).first().exec(&mut db).await?.map(OAuth2RefreshToken::from)) }
    async fn save_refresh_token(&self, token: &OAuth2RefreshToken) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); toasty::create!(OAuth2RefreshTokenModel { user_id: token.user_id as i64, user_type: token.user_type, refresh_token: token.refresh_token.clone(), access_token_id: token.access_token_id.map(|v| v as i64).unwrap_or_default(), client_id: token.client_id.clone(), scopes: token.scopes.clone().unwrap_or_default(), expires_time: token.expires_time, tenant_id: token.tenant_id as i64, creator: token.creator.clone().unwrap_or_default(), updater: token.updater.clone().unwrap_or_default() }).exec(&mut db).await?; Ok(()) }
    async fn delete_refresh_token(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match OAuth2RefreshTokenModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
