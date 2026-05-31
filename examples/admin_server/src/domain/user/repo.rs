//! 用户仓储 — toasty 实现
//!
//! User 本身 derive 了 toasty::Model，直接作为持久化模型使用，
//! 不再需要单独的 UserModel 和 From 转换。

use async_trait::async_trait;
use std::sync::Arc;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use super::{User, UserRepository, UserStatus};

#[derive(Debug)]
#[tx_comp]
pub struct ToastyUserRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl UserRepository for ToastyUserRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, anyhow::Error> {
        let mut db = self.toasty.db().clone();
        match User::get_by_id(&mut db, id).await {
            Ok(user) => Ok(Some(user)),
            Err(_) => Ok(None),
        }
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, anyhow::Error> {
        let mut db = self.toasty.db().clone();
        Ok(User::filter_by_username(username.to_string())
            .first()
            .exec(&mut db)
            .await?)
    }

    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<User>, anyhow::Error> {
        let mut db = self.toasty.db().clone();
        Ok(User::filter_by_tenant_id(tenant_id)
            .exec(&mut db)
            .await?
            .into_iter()
            .filter(|u| u.deleted == super::super::DeletedStatus::Normal)
            .collect())
    }

    async fn find_page(
        &self,
        tenant_id: u64,
        _keyword: Option<&str>,
        _status: Option<UserStatus>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<User>, u64), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        let offset = (page - 1) * page_size;

        let total = User::filter_by_tenant_id(tenant_id)
            .count()
            .exec(&mut db)
            .await? as u64;

        let users = User::filter_by_tenant_id(tenant_id)
            .offset(offset as usize)
            .limit(page_size as usize)
            .exec(&mut db)
            .await?
            .into_iter()
            .filter(|u| u.deleted == super::super::DeletedStatus::Normal)
            .collect();

        Ok((users, total))
    }

    async fn save(&self, user: &User) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if user.id == 0 {
            // 新增：toasty::create! 只需列出非默认字段
            toasty::create!(User {
                tenant_id: user.tenant_id,
                username: user.username.clone(),
                password_hash: user.password_hash.clone(),
                nickname: user.nickname.clone(),
            })
            .exec(&mut db)
            .await?;
        } else {
            // 更新：先查再改
            let mut model = User::get_by_id(&mut db, user.id)
                .await
                .map_err(|_| anyhow::anyhow!("用户不存在: {}", user.id))?;
            model.tenant_id = user.tenant_id;
            model.username = user.username.clone();
            model.password_hash = user.password_hash.clone();
            model.nickname = user.nickname.clone();
            model.remark = user.remark.clone();
            model.dept_id = user.dept_id.clone();
            model.post_ids = user.post_ids.clone();
            model.email = user.email.clone();
            model.mobile = user.mobile.clone();
            model.sex = user.sex;
            model.avatar = user.avatar.clone();
            model.status = user.status;
            model.login_ip = user.login_ip.clone();
            model.creator = user.creator.clone();
            model.updater = user.updater.clone();
            model.update().exec(&mut db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if let Ok(mut user) = User::get_by_id(&mut db, id).await {
            user.mark_deleted();
            user.update().exec(&mut db).await?;
        }
        Ok(())
    }
}
