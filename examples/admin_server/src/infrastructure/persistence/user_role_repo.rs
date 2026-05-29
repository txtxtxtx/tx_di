//! 用户角色关联仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::user_role::UserRole;

/// 用户角色关联仓储 trait
#[async_trait]
pub trait UserRoleRepository: Send + Sync {
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<UserRole>, anyhow::Error>;
    async fn find_by_role_id(&self, role_id: i64) -> Result<Vec<UserRole>, anyhow::Error>;
    async fn save(&self, ur: &UserRole) -> Result<(), anyhow::Error>;
    async fn delete_by_user_id(&self, user_id: i64) -> Result<(), anyhow::Error>;
    async fn batch_save(&self, user_id: i64, role_ids: &[i64], tenant_id: i64) -> Result<(), anyhow::Error>;
}

/// 用户角色关联仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyUserRoleRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl UserRoleRepository for ToastyUserRoleRepository {
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<UserRole>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(UserRole::filter(UserRole::user_id.eq(user_id).and(UserRole::deleted.eq(0i16)))
            .all(db)
            .await?)
    }

    async fn find_by_role_id(&self, role_id: i64) -> Result<Vec<UserRole>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(UserRole::filter(UserRole::role_id.eq(role_id).and(UserRole::deleted.eq(0i16)))
            .all(db)
            .await?)
    }

    async fn save(&self, ur: &UserRole) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        ur.clone().create(db).await?;
        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        let existing = UserRole::filter(UserRole::user_id.eq(user_id).and(UserRole::deleted.eq(0i16)))
            .all(db)
            .await?;
        for mut ur in existing {
            ur.deleted = 1;
            ur.update(db).await?;
        }
        Ok(())
    }

    async fn batch_save(&self, user_id: i64, role_ids: &[i64], tenant_id: i64) -> Result<(), anyhow::Error> {
        // 先删除旧关联
        self.delete_by_user_id(user_id).await?;
        // 再创建新关联
        for &role_id in role_ids {
            let ur = UserRole {
                id: 0,
                user_id,
                role_id,
                tenant_id,
                creator: Some("".to_string()),
                updater: Some("".to_string()),
                created_at: jiff::Timestamp::now(),
                updated_at: jiff::Timestamp::now(),
                deleted: 0,
            };
            self.save(&ur).await?;
        }
        Ok(())
    }
}
