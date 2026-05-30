//! 租户 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct TenantDto {
    pub id: u64, pub name: String, pub contact_name: Option<String>, pub contact_mobile: Option<String>,
    pub status: String, pub package_id: Option<u64>, pub account_count: i32,
    pub created_at: String, pub updated_at: String,
}

impl From<&crate::domain::tenant::Tenant> for TenantDto {
    fn from(t: &crate::domain::tenant::Tenant) -> Self {
        Self { id: t.id, name: t.name.clone(), contact_name: t.contact_name.clone(), contact_mobile: t.contact_mobile.clone(),
            status: t.status.to_string(), package_id: t.package_id, account_count: t.account_count,
            created_at: t.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: t.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string() }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest { pub name: String, pub contact_name: Option<String>, pub contact_mobile: Option<String>, pub package_id: Option<u64> }

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest { pub name: Option<String>, pub contact_name: Option<String>, pub contact_mobile: Option<String>, pub package_id: Option<u64> }
