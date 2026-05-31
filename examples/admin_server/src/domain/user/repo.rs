//! 用户仓储 — toasty 实现

use async_trait::async_trait;
use std::sync::Arc;
use toasty::Model;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use super::{Sex, User, UserRepository, UserStatus};

#[derive(Debug)]
#[tx_comp]
pub struct ToastyUserRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl UserRepository for ToastyUserRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, anyhow::Error> {
        let mut db = self.toasty.db().clone();
        match UserModel::get_by_id(&mut db, id).await {
            Ok(m) => Ok(Some(User::from(m))),
            Err(_) => Ok(None),
        }
    }
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, anyhow::Error> {
        let mut db = self.toasty.db().clone();
        Ok(UserModel::filter_by_username(username.to_string())
            .first()
            .exec(&mut db)
            .await?
            .map(User::from))
    }
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<User>, anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let models = UserModel::filter_by_tenant_id(tenant_id as i64)
            .exec(&mut db)
            .await?;
        Ok(models
            .into_iter()
            .filter(|m| m.deleted == 0)
            .map(User::from)
            .collect())
    }
    async fn find_page(
        &self,
        tenant_id: u64,
        keyword: Option<&str>,
        status: Option<UserStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<User>, u64), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let offset = (page - 1) * page_size;
        let total = UserModel::filter_by_tenant_id(tenant_id as i64)
            .count()
            .exec(&mut db)
            .await? as u64;
        let models = UserModel::filter_by_tenant_id(tenant_id as i64)
            .offset(offset as usize)
            .limit(page_size as usize)
            .exec(&mut db)
            .await?;
        Ok((
            models
                .into_iter()
                .filter(|m| m.deleted == 0)
                .map(User::from)
                .collect(),
            total,
        ))
    }
    async fn save(&self, user: &User) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if user.id == 0 {
            toasty::create!(UserModel {
                tenant_id: user.tenant_id as i64,
                username: user.username.clone(),
                password_hash: user.password_hash.clone(),
                nickname: user.nickname.clone(),
                remark: user.remark.clone().unwrap_or_default(),
                dept_id: user.dept_id.map(|v| v as i64).unwrap_or_default(),
                post_ids: user.post_ids.iter().map(|v| v.to_string()).collect(),
                email: user.email.clone().unwrap_or_default(),
                mobile: user.mobile.clone().unwrap_or_default(),
                sex: user.sex,
                avatar: user.avatar.clone().unwrap_or_default(),
                status: user.status,
                login_ip: user.login_ip.clone().unwrap_or_default(),
                creator: user.creator.clone().unwrap_or_default(),
                updater: user.updater.clone().unwrap_or_default()
            })
            .exec(&mut db)
            .await?;
        } else {
            let mut model = UserModel::get_by_id(&mut db, user.id)
                .await
                .map_err(|_| anyhow::anyhow!("not found"))?;
            model.tenant_id = user.tenant_id as i64;
            model.username = user.username.clone();
            model.password_hash = user.password_hash.clone();
            model.nickname = user.nickname.clone();
            model.remark = user.remark.clone().unwrap_or_default();
            model.dept_id = user.dept_id.map(|v| v as i64).unwrap_or_default();
            model.post_ids = user.post_ids.iter().map(|v| v.to_string()).collect();
            model.email = user.email.clone().unwrap_or_default();
            model.mobile = user.mobile.clone().unwrap_or_default();
            model.sex = user.sex;
            model.avatar = user.avatar.clone().unwrap_or_default();
            model.status = user.status;
            model.login_ip = user.login_ip.clone().unwrap_or_default();
            model.creator = user.creator.clone().unwrap_or_default();
            model.updater = user.updater.clone().unwrap_or_default();
            model.update().exec(&mut db).await?;
        }
        Ok(())
    }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        match UserModel::get_by_id(&mut db, id).await {
            Ok(mut m) => {
                m.deleted = 1;
                m.update().exec(&mut db).await?;
            }
            Err(_) => {}
        }
        Ok(())
    }
}
