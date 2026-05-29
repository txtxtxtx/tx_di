//! 菜单仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::menu::{Menu, MenuRepository};
use crate::domain::role_menu::RoleMenu;

/// 菜单仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyMenuRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl MenuRepository for ToastyMenuRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Menu>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Menu::find_by_id(db, id).await?)
    }

    async fn find_all(&self) -> Result<Vec<Menu>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Menu::filter(Menu::deleted.eq(0i16))
            .order(Menu::sort.asc())
            .all(db)
            .await?)
    }

    async fn find_by_role_ids(&self, role_ids: &[i64]) -> Result<Vec<Menu>, anyhow::Error> {
        let db = self.toasty.db();
        // 从关联表查询角色绑定的菜单 ID
        let role_menus = RoleMenu::filter(
            RoleMenu::deleted.eq(0i16)
        ).all(db).await?;
        let menu_ids: Vec<i64> = role_menus
            .iter()
            .filter(|rm| role_ids.contains(&rm.role_id))
            .map(|rm| rm.menu_id)
            .collect();
        if menu_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut menus = Vec::new();
        for mid in menu_ids {
            if let Some(menu) = Menu::find_by_id(db, mid).await? {
                if menu.deleted == 0 {
                    menus.push(menu);
                }
            }
        }
        Ok(menus)
    }

    async fn find_menu_tree(&self) -> Result<Vec<Menu>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Menu::filter(Menu::deleted.eq(0i16))
            .order(Menu::sort.asc())
            .all(db)
            .await?)
    }

    async fn save(&self, menu: &Menu) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if menu.id == 0 {
            menu.clone().create(db).await?;
        } else {
            menu.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut menu) = Menu::find_by_id(db, id).await? {
            menu.deleted = 1;
            menu.update(db).await?;
        }
        Ok(())
    }

    async fn find_by_permission(&self, permission: &str) -> Result<Option<Menu>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Menu::filter(
            Menu::permission.eq(permission).and(Menu::deleted.eq(0i16))
        ).first(db).await?)
    }
}
