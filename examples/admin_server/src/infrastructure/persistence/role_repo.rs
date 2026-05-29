//! 角色仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::role::{Role, RoleRepository};

/// 角色仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyRoleRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl RoleRepository for ToastyRoleRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Role::find_by_id(db, id).await?)
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Role>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Role::filter(Role::code.eq(code).and(Role::deleted.eq(0i16))).first(db).await?)
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Role>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Role::filter(Role::tenant_id.eq(tenant_id).and(Role::deleted.eq(0i16)))
            .all(db)
            .await?)
    }

    async fn find_page(
        &self,
        tenant_id: i64,
        keyword: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Role>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = Role::filter(Role::tenant_id.eq(tenant_id).and(Role::deleted.eq(0i16)));
        if let Some(kw) = keyword {
            stmt = stmt.filter(Role::name.like(format!("%{}%", kw)));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let roles = stmt
            .order(Role::sort.asc())
            .offset(offset)
            .limit(page_size as i64)
            .all(db)
            .await?;
        Ok((roles, total))
    }

    async fn save(&self, role: &Role) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if role.id == 0 {
            role.clone().create(db).await?;
        } else {
            role.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut role) = Role::find_by_id(db, id).await? {
            role.deleted = 1;
            role.update(db).await?;
        }
        Ok(())
    }

    async fn find_by_ids(&self, ids: &[i64]) -> Result<Vec<Role>, anyhow::Error> {
        let db = self.toasty.db();
        let mut roles = Vec::new();
        for &id in ids {
            if let Some(role) = Role::find_by_id(db, id).await? {
                roles.push(role);
            }
        }
        Ok(roles)
    }
}
