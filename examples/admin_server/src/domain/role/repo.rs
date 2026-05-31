//! 角色仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use super::{Role, RoleStatus, RoleType, RoleRepository};
use super::super::data_permission::DataScope;

#[derive(Debug, Clone, Model)]
#[table = "system_role"]
pub struct RoleModel {
    #[key] #[auto] pub id: u64, #[index] pub tenant_id: i64, pub name: String, #[unique] pub code: String,
    #[default(0i32)] pub sort: i32, pub data_scope: DataScope,
    #[serialize(json)] pub data_scope_dept_ids: Vec<String>,
    pub status: RoleStatus, pub role_type: RoleType, #[default("".to_string())] pub remark: String,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<RoleModel> for Role {
    fn from(m: RoleModel) -> Self {
        Self { id: m.id, tenant_id: m.tenant_id as u64, name: m.name, code: m.code, sort: m.sort,
            data_scope: m.data_scope, data_scope_dept_ids: m.data_scope_dept_ids.iter().filter_map(|s| s.parse().ok()).collect(),
            status: m.status, role_type: m.role_type,
            remark: if m.remark.is_empty() { None } else { Some(m.remark) }, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) },
            created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted }
    }
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyRoleRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl RoleRepository for ToastyRoleRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Role>, anyhow::Error> { let mut db = self.toasty.db().clone(); match RoleModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Role::from(m))), Err(_) => Ok(None) } }
    async fn find_by_code(&self, code: &str) -> Result<Option<Role>, anyhow::Error> { let mut db = self.toasty.db().clone(); Ok(RoleModel::filter_by_code(code.to_string()).first().exec(&mut db).await?.map(Role::from)) }
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Role>, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = RoleModel::filter_by_tenant_id(tenant_id as i64).exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.deleted == 0).map(Role::from).collect()) }
    async fn find_page(&self, tenant_id: u64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Role>, u64), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let offset = (page - 1) * page_size;
        let total = RoleModel::filter_by_tenant_id(tenant_id as i64).count().exec(&mut db).await? as u64;
        let models = RoleModel::filter_by_tenant_id(tenant_id as i64)
            .offset(offset as usize)
            .limit(page_size as usize)
            .exec(&mut db)
            .await?;
        Ok((models.into_iter().filter(|m| m.deleted == 0).map(Role::from).collect(), total))
    }
    async fn save(&self, role: &Role) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if role.id == 0 { toasty::create!(RoleModel { tenant_id: role.tenant_id as i64, name: role.name.clone(), code: role.code.clone(), sort: role.sort, data_scope: role.data_scope, data_scope_dept_ids: role.data_scope_dept_ids.iter().map(|v| v.to_string()).collect(), status: role.status, role_type: role.role_type, remark: role.remark.clone().unwrap_or_default(), creator: role.creator.clone().unwrap_or_default(), updater: role.updater.clone().unwrap_or_default() }).exec(&mut db).await?; }
        else { let mut m = RoleModel::get_by_id(&mut db, role.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.tenant_id = role.tenant_id as i64; m.name = role.name.clone(); m.code = role.code.clone(); m.sort = role.sort; m.data_scope = role.data_scope; m.data_scope_dept_ids = role.data_scope_dept_ids.iter().map(|v| v.to_string()).collect(); m.status = role.status; m.role_type = role.role_type; m.remark = role.remark.clone().unwrap_or_default(); m.creator = role.creator.clone().unwrap_or_default(); m.updater = role.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(())
    }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match RoleModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted = 1; m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
    async fn find_by_ids(&self, ids: &[u64]) -> Result<Vec<Role>, anyhow::Error> { let mut db = self.toasty.db().clone(); let mut roles = Vec::new(); for &id in ids { match RoleModel::get_by_id(&mut db, id).await { Ok(m) => roles.push(Role::from(m)), Err(_) => {} } } Ok(roles) }
}
