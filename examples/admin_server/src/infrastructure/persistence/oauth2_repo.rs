//! OAuth2 仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::oauth2::{OAuth2Client, OAuth2AccessToken, OAuth2RefreshToken};

/// OAuth2 仓储 trait
#[async_trait]
pub trait OAuth2Repository: Send + Sync {
    // Client
    async fn find_client_by_id(&self, id: i64) -> Result<Option<OAuth2Client>, anyhow::Error>;
    async fn find_client_by_client_id(&self, client_id: &str) -> Result<Option<OAuth2Client>, anyhow::Error>;
    async fn find_all_clients(&self) -> Result<Vec<OAuth2Client>, anyhow::Error>;
    async fn save_client(&self, client: &OAuth2Client) -> Result<(), anyhow::Error>;
    async fn delete_client(&self, id: i64) -> Result<(), anyhow::Error>;
    // AccessToken
    async fn find_access_token(&self, token: &str) -> Result<Option<OAuth2AccessToken>, anyhow::Error>;
    async fn save_access_token(&self, token: &OAuth2AccessToken) -> Result<(), anyhow::Error>;
    async fn delete_access_token(&self, id: i64) -> Result<(), anyhow::Error>;
    // RefreshToken
    async fn find_refresh_token(&self, token: &str) -> Result<Option<OAuth2RefreshToken>, anyhow::Error>;
    async fn save_refresh_token(&self, token: &OAuth2RefreshToken) -> Result<(), anyhow::Error>;
    async fn delete_refresh_token(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// OAuth2 仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyOAuth2Repository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl OAuth2Repository for ToastyOAuth2Repository {
    async fn find_client_by_id(&self, id: i64) -> Result<Option<OAuth2Client>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(OAuth2Client::find_by_id(db, id).await?)
    }

    async fn find_client_by_client_id(&self, client_id: &str) -> Result<Option<OAuth2Client>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(OAuth2Client::filter(
            OAuth2Client::client_id.eq(client_id).and(OAuth2Client::deleted.eq(0i16))
        ).first(db).await?)
    }

    async fn find_all_clients(&self) -> Result<Vec<OAuth2Client>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(OAuth2Client::filter(OAuth2Client::deleted.eq(0i16)).all(db).await?)
    }

    async fn save_client(&self, client: &OAuth2Client) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if client.id == 0 {
            client.clone().create(db).await?;
        } else {
            client.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete_client(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut c) = OAuth2Client::find_by_id(db, id).await? {
            c.deleted = 1;
            c.update(db).await?;
        }
        Ok(())
    }

    async fn find_access_token(&self, token: &str) -> Result<Option<OAuth2AccessToken>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(OAuth2AccessToken::filter(
            OAuth2AccessToken::access_token.eq(token).and(OAuth2AccessToken::deleted.eq(0i16))
        ).first(db).await?)
    }

    async fn save_access_token(&self, token: &OAuth2AccessToken) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        token.clone().create(db).await?;
        Ok(())
    }

    async fn delete_access_token(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut t) = OAuth2AccessToken::find_by_id(db, id).await? {
            t.deleted = 1;
            t.update(db).await?;
        }
        Ok(())
    }

    async fn find_refresh_token(&self, token: &str) -> Result<Option<OAuth2RefreshToken>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(OAuth2RefreshToken::filter(
            OAuth2RefreshToken::refresh_token.eq(token).and(OAuth2RefreshToken::deleted.eq(0i16))
        ).first(db).await?)
    }

    async fn save_refresh_token(&self, token: &OAuth2RefreshToken) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        token.clone().create(db).await?;
        Ok(())
    }

    async fn delete_refresh_token(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut t) = OAuth2RefreshToken::find_by_id(db, id).await? {
            t.deleted = 1;
            t.update(db).await?;
        }
        Ok(())
    }
}
