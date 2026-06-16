use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::department::model::aggregate::Department;
use admin_domain::department::model::value_object::DeptQuery;
use admin_domain::department::repository::DepartmentRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::SysDepartment;
use crate::user::model::SysUserDept;
use crate::common::{Status, Deleted};

/// Toasty 实现的 DepartmentRepository
#[tx_comp(as_trait = dyn DepartmentRepository)]
pub struct ToastyDepartmentRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyDepartmentRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(d: &SysDepartment) -> Department {
        Department::restore(
            d.id as u64,
            d.name.clone(),
            d.parent_id as u64,
            d.sort,
            if d.leader_user_id == 0 { None } else { Some(d.leader_user_id as u64) },
            if d.phone.is_empty() { None } else { Some(d.phone.clone()) },
            if d.email.is_empty() { None } else { Some(d.email.clone()) },
            i32::from(d.status),
            d.tenant_id,
            AuditFields {
                creator: if d.creator.is_empty() { None } else { Some(d.creator.clone()) },
                create_time: d.created_at.parse().unwrap_or_default(),
                updater: if d.updater.is_empty() { None } else { Some(d.updater.clone()) },
                update_time: d.updated_at.parse().unwrap_or_default(),
                deleted: if d.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl DepartmentRepository for ToastyDepartmentRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Department>> {
        let mut db = self.plugin.db().clone();
        match SysDepartment::get_by_id(&mut db, id as i64).await {
            Ok(d) if d.deleted == Deleted::No => Ok(Some(Self::to_domain(&d))),
            _ => Ok(None),
        }
    }

    async fn find_all(&self, query: &DeptQuery) -> AppResult<Vec<Department>> {
        let mut db = self.plugin.db().clone();
        let all = SysDepartment::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|d| d.deleted == Deleted::No)
            .filter(|d| {
                if let Some(ref name) = query.name {
                    if !d.name.contains(name.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if i32::from(d.status) != status { return false; }
                }
                true
            })
            .map(Self::to_domain)
            .collect())
    }

    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Department>> {
        let mut db = self.plugin.db().clone();
        let all = SysDepartment::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|d| d.deleted == Deleted::No && ids.contains(&(d.id as u64)))
            .map(Self::to_domain)
            .collect())
    }

    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Department>> {
        let mut db = self.plugin.db().clone();
        let all = SysDepartment::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|d| d.deleted == Deleted::No && d.parent_id == parent_id as i64)
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, dept: &Department) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysDepartment::create()
            .id(dept.id as i64)
            .name(dept.name.clone())
            .parent_id(dept.parent_id as i64)
            .sort(dept.sort)
            .leader_user_id(dept.leader_user_id.map(|id| id as i64).unwrap_or(0))
            .phone(dept.phone.clone().unwrap_or_default())
            .email(dept.email.clone().unwrap_or_default())
            .status(Status::from(dept.status))
            .tenant_id(dept.tenant_id)
            .creator(dept.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(dept.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(dept.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn update(&self, dept: &Department) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysDepartment::get_by_id(&mut db, dept.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(dept.name.clone())
            .parent_id(dept.parent_id as i64)
            .sort(dept.sort)
            .leader_user_id(dept.leader_user_id.map(|id| id as i64).unwrap_or(0))
            .phone(dept.phone.clone().unwrap_or_default())
            .email(dept.email.clone().unwrap_or_default())
            .status(Status::from(dept.status))
            .tenant_id(dept.tenant_id)
            .updater(dept.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(dept.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut dept = SysDepartment::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        dept.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn has_children(&self, parent_id: u64) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let all = SysDepartment::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all.iter().any(|d| d.deleted == Deleted::No && d.parent_id == parent_id as i64))
    }

    async fn has_users(&self, dept_id: u64) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let user_depts = SysUserDept::filter_by_dept_id(dept_id as i64)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(!user_depts.is_empty())
    }
}
