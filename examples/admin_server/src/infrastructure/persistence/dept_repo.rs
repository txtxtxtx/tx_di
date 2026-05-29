//! 部门仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::dept::Dept;

/// 部门仓储 trait（领域层定义，此处实现）
#[async_trait]
pub trait DeptRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Dept>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Dept>, anyhow::Error>;
    async fn find_by_parent_id(&self, parent_id: i64, tenant_id: i64) -> Result<Vec<Dept>, anyhow::Error>;
    async fn save(&self, dept: &Dept) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// 部门仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyDeptRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl DeptRepository for ToastyDeptRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Dept>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Dept::find_by_id(db, id).await?)
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Dept>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Dept::filter(Dept::tenant_id.eq(tenant_id).and(Dept::deleted.eq(0i16)))
            .order(Dept::sort.asc())
            .all(db)
            .await?)
    }

    async fn find_by_parent_id(&self, parent_id: i64, tenant_id: i64) -> Result<Vec<Dept>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Dept::filter(
            Dept::parent_id.eq(parent_id)
                .and(Dept::tenant_id.eq(tenant_id))
                .and(Dept::deleted.eq(0i16))
        )
        .order(Dept::sort.asc())
        .all(db)
        .await?)
    }

    async fn save(&self, dept: &Dept) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if dept.id == 0 {
            dept.clone().create(db).await?;
        } else {
            dept.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut dept) = Dept::find_by_id(db, id).await? {
            dept.deleted = 1;
            dept.update(db).await?;
        }
        Ok(())
    }
}
