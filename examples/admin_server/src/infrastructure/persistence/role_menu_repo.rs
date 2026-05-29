//! 角色菜单关联仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::role_menu::RoleMenu;

/// 角色菜单关联仓储 trait
#[async_trait]
pub trait RoleMenuRepository: Send + Sync {
    async fn find_by_role_id(&self, role_id: i64) -> Result<Vec<RoleMenu>, anyhow::Error>;
    async fn save(&self, rm: &RoleMenu) -> Result<(), anyhow::Error>;
    async fn delete_by_role_id(&self, role_id: i64) -> Result<(), anyhow::Error>;
    async fn batch_save(&self, role_id: i64, menu_ids: &[i64], tenant_id: i64) -> Result<(), anyhow::Error>;
}

/// 角色菜单关联仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyRoleMenuRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl RoleMenuRepository for ToastyRoleMenuRepository {
    async fn find_by_role_id(&self, role_id: i64) -> Result<Vec<RoleMenu>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(RoleMenu::filter(RoleMenu::role_id.eq(role_id).and(RoleMenu::deleted.eq(0i16)))
            .all(db)
            .await?)
    }

    async fn save(&self, rm: &RoleMenu) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        rm.clone().create(db).await?;
        Ok(())
    }

    async fn delete_by_role_id(&self, role_id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        let existing = RoleMenu::filter(RoleMenu::role_id.eq(role_id).and(RoleMenu::deleted.eq(0i16)))
            .all(db)
            .await?;
        for mut rm in existing {
            rm.deleted = 1;
            rm.update(db).await?;
        }
        Ok(())
    }

    async fn batch_save(&self, role_id: i64, menu_ids: &[i64], tenant_id: i64) -> Result<(), anyhow::Error> {
        self.delete_by_role_id(role_id).await?;
        for &menu_id in menu_ids {
            let rm = RoleMenu {
                id: 0,
                role_id,
                menu_id,
                tenant_id,
                creator: Some("".to_string()),
                updater: Some("".to_string()),
                created_at: jiff::Timestamp::now(),
                updated_at: jiff::Timestamp::now(),
                deleted: 0,
            };
            self.save(&rm).await?;
        }
        Ok(())
    }
}
