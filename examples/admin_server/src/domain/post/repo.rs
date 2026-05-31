//! 岗位仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{Post, PostRepository};
use super::super::dept::CommonStatus;

#[derive(Debug, Clone, Model)]
#[table = "system_post"]
pub struct PostModel {
    #[key] #[auto] pub id: u64, pub code: String, pub name: String,
    #[default(0i32)] pub sort: i32, pub status: CommonStatus, #[default("".to_string())] pub remark: String, #[index] pub tenant_id: i64,
    #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<PostModel> for Post {
    fn from(m: PostModel) -> Self { Self { id: m.id, code: m.code, name: m.name, sort: m.sort, status: m.status, remark: if m.remark.is_empty() { None } else { Some(m.remark) }, tenant_id: m.tenant_id as u64, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } }
}

#[derive(Debug)] #[tx_comp]
pub struct ToastyPostRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl PostRepository for ToastyPostRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<Post>, anyhow::Error> { let mut db = self.toasty.db().clone(); match PostModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(Post::from(m))), Err(_) => Ok(None) } }
    async fn find_by_tenant(&self, tenant_id: u64) -> Result<Vec<Post>, anyhow::Error> { let mut db = self.toasty.db().clone(); let models = PostModel::filter_by_tenant_id(tenant_id as i64).exec(&mut db).await?; Ok(models.into_iter().filter(|m| m.deleted == 0).map(Post::from).collect()) }
    async fn save(&self, post: &Post) -> Result<(), anyhow::Error> {
        let mut db = self.toasty.db().clone();
        if post.id == 0 { toasty::create!(PostModel { code: post.code.clone(), name: post.name.clone(), sort: post.sort, status: post.status, remark: post.remark.clone().unwrap_or_default(), tenant_id: post.tenant_id as i64, creator: post.creator.clone().unwrap_or_default(), updater: post.updater.clone().unwrap_or_default() }).exec(&mut db).await?; }
        else { let mut m = PostModel::get_by_id(&mut db, post.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.code = post.code.clone(); m.name = post.name.clone(); m.sort = post.sort; m.status = post.status; m.remark = post.remark.clone().unwrap_or_default(); m.tenant_id = post.tenant_id as i64; m.creator = post.creator.clone().unwrap_or_default(); m.updater = post.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(())
    }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match PostModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted = 1; m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
