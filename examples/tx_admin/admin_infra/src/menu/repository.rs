use std::sync::Arc;
use async_trait::async_trait;

use admin_domain::menu::model::aggregate::Menu;
use admin_domain::menu::model::value_object::MenuQuery;
use admin_domain::menu::repository::MenuRepository;
use admin_domain::shared::model::value_object::DeletedStatus;
use admin_domain::shared::model::AuditFields;
use admin_domain::shared::repository::RepositoryError;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use tx_error::AppResult;

use super::model::SysMenu;
use crate::common::{Status, Deleted};

/// Toasty 实现的 MenuRepository
#[tx_comp(as_trait = dyn MenuRepository)]
pub struct ToastyMenuRepository {
    plugin: Arc<ToastyPlugin>,
}

impl ToastyMenuRepository {
    pub fn new(plugin: Arc<ToastyPlugin>) -> Self {
        Self { plugin }
    }

    fn to_domain(m: &SysMenu) -> Menu {
        Menu::restore(
            m.id as u64,
            m.name.clone(),
            m.permission.clone(),
            m.types,
            m.sort,
            m.parent_id as u64,
            if m.route_path.is_empty() { None } else { Some(m.route_path.clone()) },
            if m.icon.is_empty() { None } else { Some(m.icon.clone()) },
            if m.component.is_empty() { None } else { Some(m.component.clone()) },
            if m.component_name.is_empty() { None } else { Some(m.component_name.clone()) },
            i32::from(m.status),
            m.visible,
            m.keep_alive,
            m.tenant_id,
            AuditFields {
                creator: if m.creator.is_empty() { None } else { Some(m.creator.clone()) },
                create_time: m.created_at.parse().unwrap_or_default(),
                updater: if m.updater.is_empty() { None } else { Some(m.updater.clone()) },
                update_time: m.updated_at.parse().unwrap_or_default(),
                deleted: if m.deleted == Deleted::Yes { DeletedStatus::Deleted } else { DeletedStatus::Normal },
            },
        )
    }
}

#[async_trait]
impl MenuRepository for ToastyMenuRepository {
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Menu>> {
        let mut db = self.plugin.db().clone();
        match SysMenu::get_by_id(&mut db, id as i64).await {
            Ok(m) if m.deleted == Deleted::No => Ok(Some(Self::to_domain(&m))),
            _ => Ok(None),
        }
    }

    async fn find_all(&self, query: &MenuQuery) -> AppResult<Vec<Menu>> {
        let mut db = self.plugin.db().clone();
        let all = SysMenu::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|m| m.deleted == Deleted::No)
            .filter(|m| {
                if let Some(ref name) = query.name {
                    if !m.name.contains(name.as_str()) { return false; }
                }
                if let Some(status) = query.status {
                    if i32::from(m.status) != status { return false; }
                }
                if let Some(types) = query.types {
                    if m.types != types { return false; }
                }
                true
            })
            .map(Self::to_domain)
            .collect())
    }

    async fn find_by_ids(&self, ids: &[u64]) -> AppResult<Vec<Menu>> {
        let mut db = self.plugin.db().clone();
        let all = SysMenu::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|m| m.deleted == Deleted::No && ids.contains(&(m.id as u64)))
            .map(Self::to_domain)
            .collect())
    }

    async fn find_by_parent_id(&self, parent_id: u64) -> AppResult<Vec<Menu>> {
        let mut db = self.plugin.db().clone();
        let all = SysMenu::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all
            .iter()
            .filter(|m| m.deleted == Deleted::No && m.parent_id == parent_id as i64)
            .map(Self::to_domain)
            .collect())
    }

    async fn insert(&self, menu: &Menu) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let now = jiff::Timestamp::now().to_string();
        SysMenu::create()
            .id(menu.id as i64)
            .name(menu.name.clone())
            .permission(menu.permission.clone())
            .types(menu.types)
            .sort(menu.sort)
            .parent_id(menu.parent_id as i64)
            .route_path(menu.path.clone().unwrap_or_default())
            .icon(menu.icon.clone().unwrap_or_default())
            .component(menu.component.clone().unwrap_or_default())
            .component_name(menu.component_name.clone().unwrap_or_default())
            .status(Status::from(menu.status))
            .visible(menu.visible)
            .keep_alive(menu.keep_alive)
            .tenant_id(menu.tenant_id)
            .creator(menu.audit.creator.clone().unwrap_or_default())
            .created_at(now.clone())
            .updater(menu.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(menu.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn update(&self, menu: &Menu) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut existing = SysMenu::get_by_id(&mut db, menu.id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        let now = jiff::Timestamp::now().to_string();
        existing
            .update()
            .name(menu.name.clone())
            .permission(menu.permission.clone())
            .types(menu.types)
            .sort(menu.sort)
            .parent_id(menu.parent_id as i64)
            .route_path(menu.path.clone().unwrap_or_default())
            .icon(menu.icon.clone().unwrap_or_default())
            .component(menu.component.clone().unwrap_or_default())
            .component_name(menu.component_name.clone().unwrap_or_default())
            .status(Status::from(menu.status))
            .visible(menu.visible)
            .keep_alive(menu.keep_alive)
            .tenant_id(menu.tenant_id)
            .updater(menu.audit.updater.clone().unwrap_or_default())
            .updated_at(now)
            .deleted(Deleted::from(menu.audit.deleted))
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn soft_delete(&self, id: u64) -> AppResult<()> {
        let mut db = self.plugin.db().clone();
        let mut menu = SysMenu::get_by_id(&mut db, id as i64)
            .await
            .map_err(|_| RepositoryError::NotFound)?;

        menu.update()
            .deleted(Deleted::Yes)
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;
        Ok(())
    }

    async fn has_children(&self, parent_id: u64) -> AppResult<bool> {
        let mut db = self.plugin.db().clone();
        let all = SysMenu::all()
            .exec(&mut db)
            .await
            .map_err(|_| RepositoryError::Database)?;

        Ok(all.iter().any(|m| m.deleted == Deleted::No && m.parent_id == parent_id as i64))
    }
}
