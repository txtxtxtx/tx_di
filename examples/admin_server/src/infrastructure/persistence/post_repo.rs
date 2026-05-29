//! 岗位仓储 — toasty 0.6 实现

use std::sync::Arc;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;

use crate::domain::post::Post;

/// 岗位仓储 trait
#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Post>, anyhow::Error>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Post>, anyhow::Error>;
    async fn find_page(&self, tenant_id: i64, keyword: Option<&str>, page: u64, page_size: u64) -> Result<(Vec<Post>, u64), anyhow::Error>;
    async fn save(&self, post: &Post) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: i64) -> Result<(), anyhow::Error>;
}

/// 岗位仓储 — toasty 实现
#[derive(Debug)]
#[tx_comp]
pub struct ToastyPostRepository {
    pub toasty: Arc<ToastyPlugin>,
}

#[async_trait]
impl PostRepository for ToastyPostRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Post>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Post::find_by_id(db, id).await?)
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Post>, anyhow::Error> {
        let db = self.toasty.db();
        Ok(Post::filter(Post::tenant_id.eq(tenant_id).and(Post::deleted.eq(0i16)))
            .order(Post::sort.asc())
            .all(db)
            .await?)
    }

    async fn find_page(
        &self,
        tenant_id: i64,
        keyword: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Post>, u64), anyhow::Error> {
        let db = self.toasty.db();
        let mut stmt = Post::filter(Post::tenant_id.eq(tenant_id).and(Post::deleted.eq(0i16)));
        if let Some(kw) = keyword {
            stmt = stmt.filter(Post::name.like(format!("%{}%", kw)));
        }
        let total = stmt.clone().count(db).await? as u64;
        let offset = ((page - 1) * page_size) as i64;
        let posts = stmt.order(Post::sort.asc()).offset(offset).limit(page_size as i64).all(db).await?;
        Ok((posts, total))
    }

    async fn save(&self, post: &Post) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if post.id == 0 {
            post.clone().create(db).await?;
        } else {
            post.clone().update(db).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), anyhow::Error> {
        let db = self.toasty.db();
        if let Some(mut post) = Post::find_by_id(db, id).await? {
            post.deleted = 1;
            post.update(db).await?;
        }
        Ok(())
    }
}
