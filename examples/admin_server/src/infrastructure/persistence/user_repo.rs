//! 用户仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::data_permission::DataScope;
use crate::domain::user::{User, UserStatus, UserRepository};

/// 用户仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyUserRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl UserRepository for ToastyUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(User::find_by_id(db, id).await?)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(User::filter(User::username.eq(username)).first(db).await?)
    }

    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        _data_scope: &DataScope,
        _current_user_id: i64,
    ) -> Result<Vec<User>, anyhow::Error> {
        let db = self.toasty.db();
        // TODO: 根据 data_scope 过滤
        Ok(User::filter(User::tenant_id.eq(tenant_id)).all(db).await?)
    }

    async fn find_page(
        &self,
        tenant_id: i64,
        keyword: Option<&str>,
        status: Option<UserStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<User>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = User::filter(User::tenant_id.eq(tenant_id).and(User::deleted.eq(0i16)));
        if let Some(kw) = keyword {
            stmt = stmt.filter(User::username.like(format!("%{}%", kw)));
        }
        if let Some(s) = status {
            stmt = stmt.filter(User::status.eq(s.to_string()));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let users = stmt
            .order(User::id.desc())
            .offset(offset)
            .limit(page_size as i64)
            .all(db)
            .await?;
        Ok((users, total))
    }

    async fn save(&self, user: &User) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if user.id == 0 {
            user.clone().create(db).await?;
        } else {
            user.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut user) = User::find_by_id(db, id).await? {
            user.deleted = 1;
            user.update(db).await?;
        }
        Ok(())
    }
}
