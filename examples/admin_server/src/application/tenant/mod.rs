//! 租户应用服务

use std::sync::Arc;
use tx_di_core::tx_comp;

use crate::domain::tenant::{Tenant, TenantId, TenantPackage, TenantStatus};
use crate::infrastructure::persistence::InMemoryTenantRepository;

/// 创建租户请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub code: String,
    pub contact_name: Option<String>,
    pub contact_mobile: Option<String>,
    pub package_id: Option<String>,
    pub domain: Option<String>,
    pub expire_time: Option<String>,
}

/// 更新租户请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_mobile: Option<String>,
    pub status: Option<TenantStatus>,
    pub package_id: Option<String>,
    pub domain: Option<String>,
    pub expire_time: Option<String>,
}

/// 租户应用服务
#[derive(Debug)]
#[tx_comp]
pub struct TenantService {
    pub tenant_repo: Arc<InMemoryTenantRepository>,
}

impl TenantService {
    /// 查询租户列表
    pub async fn list_tenants(
        &self,
        keyword: Option<&str>,
        status: Option<TenantStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Tenant>, u64), anyhow::Error> {
        self.tenant_repo.find_page(keyword, status, page, page_size).await
    }

    /// 创建租户
    pub async fn create_tenant(
        &self,
        req: CreateTenantRequest,
    ) -> Result<Tenant, anyhow::Error> {
        if self.tenant_repo.find_by_code(&req.code).await?.is_some() {
            return Err(anyhow::anyhow!("租户编码已存在"));
        }

        let tenant_id = uuid::Uuid::new_v4().to_string();
        let mut tenant = Tenant::new(tenant_id, req.name, req.code);
        tenant.contact_name = req.contact_name;
        tenant.contact_mobile = req.contact_mobile;
        tenant.package_id = req.package_id;
        tenant.domain = req.domain;

        if let Some(expire_str) = req.expire_time {
            tenant.expire_time = Some(
                chrono::DateTime::parse_from_rfc3339(&expire_str)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or(chrono::Utc::now()),
            );
        }

        self.tenant_repo.save(&tenant).await?;
        tracing::info!(tenant_id = %tenant.id, tenant_name = %tenant.name, "租户已创建");
        Ok(tenant)
    }

    /// 更新租户
    pub async fn update_tenant(
        &self,
        tenant_id: &str,
        req: UpdateTenantRequest,
    ) -> Result<Tenant, anyhow::Error> {
        let mut tenant = self
            .tenant_repo
            .find_by_id(&tenant_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("租户不存在"))?;

        if let Some(name) = req.name {
            tenant.name = name;
        }
        tenant.contact_name = req.contact_name.or(tenant.contact_name);
        tenant.contact_mobile = req.contact_mobile.or(tenant.contact_mobile);
        if let Some(status) = req.status {
            tenant.status = status;
        }
        if let Some(package_id) = req.package_id {
            tenant.package_id = Some(package_id);
        }
        if let Some(domain) = req.domain {
            tenant.domain = Some(domain);
        }
        if let Some(expire_str) = req.expire_time {
            tenant.expire_time = Some(
                chrono::DateTime::parse_from_rfc3339(&expire_str)
                    .map(|d| d.with_timezone(&chrono::Utc))
                    .unwrap_or(chrono::Utc::now()),
            );
        }

        self.tenant_repo.save(&tenant).await?;
        Ok(tenant)
    }

    /// 删除租户
    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<(), anyhow::Error> {
        self.tenant_repo
            .find_by_id(&tenant_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("租户不存在"))?;

        self.tenant_repo.delete(&tenant_id.to_string()).await?;
        tracing::info!(tenant_id = %tenant_id, "租户已删除");
        Ok(())
    }

    /// 获取所有套餐
    pub async fn list_packages(&self) -> Result<Vec<TenantPackage>, anyhow::Error> {
        self.tenant_repo.find_all_packages().await
    }
}
