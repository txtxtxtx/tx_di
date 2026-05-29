//! 用户岗位关联仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::user_post::UserPost;

/// 用户岗位关联仓储 trait
#[async_trait]
pub trait UserPostRepository: Send + Sync {
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<UserPost>, anyhow::Error>;
    async fn save(&self, up: &UserPost) -> Result<(), anyhow::Error>;
    async fn delete_by_user_id(&self, user_id: i64) -> Result<(), anyhow::Error>;
    async fn batch_save(&self, user_id: i64, post_ids: &[i64], tenant_id: i64) -> Result<(), anyhow::Error>;
}

/// 用户岗位关联仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyUserPostRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl UserPostRepository for ToastyUserPostRepository {
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<UserPost>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(UserPost::filter(UserPost::user_id.eq(user_id).and(UserPost::deleted.eq(0i16)))
            .all(db)
            .await?)
    }

    async fn save(&self, up: &UserPost) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        up.clone().create(db).await?;
        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        let existing = UserPost::filter(UserPost::user_id.eq(user_id).and(UserPost::deleted.eq(0i16)))
            .all(db)
            .await?;
        for mut up in existing {
            up.deleted = 1;
            up.update(db).await?;
        }
        Ok(())
    }

    async fn batch_save(&self, user_id: i64, post_ids: &[i64], tenant_id: i64) -> Result<(), anyhow::Error> {
        self.delete_by_user_id(user_id).await?;
        for &post_id in post_ids {
            let up = UserPost {
                id: 0,
                user_id,
                post_id,
                tenant_id,
                creator: Some("".to_string()),
                updater: Some("".to_string()),
                created_at: jiff::Timestamp::now(),
                updated_at: jiff::Timestamp::now(),
                deleted: 0,
            };
            self.save(&up).await?;
        }
        Ok(())
    }
}
