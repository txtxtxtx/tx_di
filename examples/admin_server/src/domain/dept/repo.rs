//! 部门仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{Dept, CommonStatus, DeptRepository};

#[derive(Debug, Clone, Model)]
#[table = "system_dept"]
pub struct DeptModel {
    #[key] #[auto] pub id: u64, #[index] pub tenant_id: i64, pub name: String,
    #[default(0u64)] pub parent_id: u64, #[default(0i32)] pub sort: i32,
    #[default(0i64)] pub leader_user_id: i64, #[default("".to_string())] pub phone: String, #[default("".to_string())] pub email: String,
    pub status: CommonStatus, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<DeptModel> for Dept {
    fn from(m: DeptModel) -> Self { Self { id: m.id, tenant_id: m.tenant_id as u64, name: m.name, parent_id: m.parent_id, sort: m.sort, leader_user_id: if m.leader_user_id == 0 { None } else { Some(m.leader_user_id as u64) }, phone: if m.phone.is_empty() { None } else { Some(m.phone) }, email: if m.email.is_empty() { None } else { Some(m.email) }, status: m.status, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } }
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyDeptRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl DeptRepository for ToastyDeptRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Dept>, anyhow::Error> { let mut db = self.toasty.db().clone(); match DeptModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Dept::from(m))), Err(_) => Ok(None) } }
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Dept>, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = DeptModel::filter_by_tenant_id(tenant_id as i64).exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.deleted == 0).map(Dept::from).collect()) }
    async fn save(&self, dept: &Dept) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if dept.id == 0 { toasty::create!(DeptModel { tenant_id: dept.tenant_id as i64, name: dept.name.clone(), parent_id: dept.parent_id, sort: dept.sort, leader_user_id: dept.leader_user_id.map(|v| v as i64).unwrap_or_default(), phone: dept.phone.clone().unwrap_or_default(), email: dept.email.clone().unwrap_or_default(), status: dept.status, creator: dept.creator.clone().unwrap_or_default(), updater: dept.updater.clone().unwrap_or_default() }).exec(&mut db).await?; }
        else { let mut m = DeptModel::get_by_id(&mut db, dept.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.tenant_id = dept.tenant_id as i64; m.name = dept.name.clone(); m.parent_id = dept.parent_id; m.sort = dept.sort; m.leader_user_id = dept.leader_user_id.map(|v| v as i64).unwrap_or_default(); m.phone = dept.phone.clone().unwrap_or_default(); m.email = dept.email.clone().unwrap_or_default(); m.status = dept.status; m.creator = dept.creator.clone().unwrap_or_default(); m.updater = dept.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(())
    }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match DeptModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted = 1; m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
