//! 租户仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::tenant::{Tenant, TenantPackage, TenantRepository, TenantStatus};

/// 租户仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyTenantRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl TenantRepository for ToastyTenantRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Tenant>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Tenant::find_by_id(db, id).await?)
    }

    async fn find_by_domain(&self, _domain: &str) -> Result<Option<Tenant>, anyhow::Error> {
        let db = self.toasty.db();
        // websites 是 JSON 数组，需要全量过滤
        let tenants = Tenant::filter(Tenant::deleted.eq(0i16)).all(db).await?;
        Ok(tenants.into_iter().find(|t| t.websites.iter().any(|w| w == _domain)))
    }

    async fn find_page(
        &self,
        keyword: Option<&str>,
        status: Option<TenantStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Tenant>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = Tenant::filter(Tenant::deleted.eq(0i16));
        if let Some(kw) = keyword {
            stmt = stmt.filter(Tenant::name.like(format!("%{}%", kw)));
        }
        if let Some(s) = status {
            stmt = stmt.filter(Tenant::status.eq(s.to_string()));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let tenants = stmt
            .order(Tenant::id.desc())
            .offset(offset)
            .limit(page_size as i64)
            .all(db)
            .await?;
        Ok((tenants, total))
    }

    async fn save(&self, tenant: &Tenant) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if tenant.id == 0 {
            tenant.clone().create(db).await?;
        } else {
            tenant.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut tenant) = Tenant::find_by_id(db, id).await? {
            tenant.deleted = 1;
            tenant.update(db).await?;
        }
        Ok(())
    }

    async fn find_by_code(&self, _code: &str) -> Result<Option<Tenant>, anyhow::Error> {
        // Tenant 没有 code 字段（ruoyi 中租户用 name 区分），兼容旧接口
        let db = self.toasty.db();
        Ok(Tenant::filter(Tenant::name.eq(_code).and(Tenant::deleted.eq(0i16)))
            .first(db)
            .await?)
    }

    async fn find_package_by_id(&self, id: i64) -> Result<Option<TenantPackage>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(TenantPackage::find_by_id(db, id).await?)
    }

    async fn find_all_packages(&self) -> Result<Vec<TenantPackage>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(TenantPackage::filter(TenantPackage::deleted.eq(0i16))
            .all(db)
            .await?)
    }
}
