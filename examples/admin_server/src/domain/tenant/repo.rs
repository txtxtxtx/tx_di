//! 租户仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use super::{Tenant, TenantPackage, TenantStatus, TenantRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_tenant"]
pub struct TenantModel {
    #[key] #[auto] pub id: u64, pub name: String,
    #[default(0i64)] pub contact_user_id: i64, #[default("".to_string())] pub contact_name: String, #[default("".to_string())] pub contact_mobile: String,
    pub status: String, #[default(Vec::new())] #[serialize(json)] pub websites: Vec<String>, #[default(0i64)] pub package_id: i64,
    #[default(0i32)] pub account_count: i32, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

#[derive(Debug, Clone, Model)]
#[table = "system_tenant_package"]
pub struct TenantPackageModel {
    #[key] #[auto] pub id: u64, pub name: String, pub status: String, #[default("".to_string())] pub remark: String,
    #[default(Vec::new())] #[serialize(json)] pub menu_ids: Vec<String>, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<TenantModel> for Tenant {
    fn from(m: TenantModel) -> Self { Self { id: m.id, name: m.name, contact_user_id: if m.contact_user_id == 0 { None } else { Some(m.contact_user_id as u64) }, contact_name: if m.contact_name.is_empty() { None } else { Some(m.contact_name) }, contact_mobile: if m.contact_mobile.is_empty() { None } else { Some(m.contact_mobile) }, status: if m.status == "disabled" { TenantStatus::Disabled } else { TenantStatus::Active }, websites: m.websites, package_id: if m.package_id == 0 { None } else { Some(m.package_id as u64) }, expire_time: None, account_count: m.account_count, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } }
}
impl From<TenantPackageModel> for TenantPackage {
    fn from(m: TenantPackageModel) -> Self { Self { id: m.id, name: m.name, status: if m.status == "disabled" { TenantStatus::Disabled } else { TenantStatus::Active }, remark: if m.remark.is_empty() { None } else { Some(m.remark) }, menu_ids: m.menu_ids.iter().filter_map(|s| s.parse().ok()).collect(), creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } }
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyTenantRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl TenantRepository for ToastyTenantRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Tenant>, anyhow::Error> { let mut db = self.toasty.db().clone(); match TenantModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Tenant::from(m))), Err(_) => Ok(None) } }
    async fn find_page(&self, keyword: Option<&str>, status: Option<TenantStatus>, page: u64, page_size: u64) -> Result<(Vec<Tenant>, u64), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let offset = (page - 1) * page_size;
        let total = TenantModel::all().count().exec(&mut db).await? as u64;
        let models = TenantModel::all()
            .offset(offset as usize)
            .limit(page_size as usize)
            .exec(&mut db)
            .await?;
        Ok((models.into_iter().filter(|m| m.deleted == 0).map(Tenant::from).collect(), total))
    }
    async fn save(&self, tenant: &Tenant) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if tenant.id == 0 { toasty::create!(TenantModel { name: tenant.name.clone(), contact_user_id: tenant.contact_user_id.map(|v| v as i64).unwrap_or_default(), contact_name: tenant.contact_name.clone().unwrap_or_default(), contact_mobile: tenant.contact_mobile.clone().unwrap_or_default(), status: tenant.status.to_string(), websites: tenant.websites.clone(), package_id: tenant.package_id.map(|v| v as i64).unwrap_or_default(), account_count: tenant.account_count, creator: tenant.creator.clone().unwrap_or_default(), updater: tenant.updater.clone().unwrap_or_default() }).exec(&mut db).await?; }
        else { let mut m = TenantModel::get_by_id(&mut db, tenant.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.name = tenant.name.clone(); m.contact_user_id = tenant.contact_user_id.map(|v| v as i64).unwrap_or_default(); m.contact_name = tenant.contact_name.clone().unwrap_or_default(); m.contact_mobile = tenant.contact_mobile.clone().unwrap_or_default(); m.status = tenant.status.to_string(); m.websites = tenant.websites.clone(); m.package_id = tenant.package_id.map(|v| v as i64).unwrap_or_default(); m.account_count = tenant.account_count; m.creator = tenant.creator.clone().unwrap_or_default(); m.updater = tenant.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(())
    }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match TenantModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted = 1; m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn find_all_packages(&self) -> Result<Vec<TenantPackage>, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = TenantPackageModel::all().exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.deleted == 0).map(TenantPackage::from).collect()) }
}
