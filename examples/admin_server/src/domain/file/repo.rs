//! 文件仓储 — toasty 实现

use std::sync::Arc;
use toasty::Model;
use async_trait::async_trait;
use tx_di_core::tx_comp;
use tx_di_toasty::ToastyPlugin;
use super::{File, FileRepository};

#[derive(Debug, Clone, Model)]
#[table = "infra_file"]
pub struct FileModel {
    #[key] #[auto] pub id: u64, #[default(0i64)] pub config_id: i64, #[default("".to_string())] pub name: String,
    #[column("path")] pub file_path: String, pub url: String, #[default("".to_string())] pub file_type: String,
    #[default(0i32)] pub size: i32, #[default("".to_string())] pub creator: String, #[default("".to_string())] pub updater: String,
    #[default(jiff::Timestamp::now())] pub created_at: jiff::Timestamp,
    #[update(jiff::Timestamp::now())] pub updated_at: jiff::Timestamp,
    #[default(0u8)] pub deleted: u8,
}

impl From<FileModel> for File { fn from(m: FileModel) -> Self { Self { id: m.id, config_id: if m.config_id == 0 { None } else { Some(m.config_id as u64) }, name: if m.name.is_empty() { None } else { Some(m.name) }, file_path: m.file_path, url: m.url, file_type: if m.file_type.is_empty() { None } else { Some(m.file_type) }, size: m.size, creator: if m.creator.is_empty() { None } else { Some(m.creator) }, updater: if m.updater.is_empty() { None } else { Some(m.updater) }, created_at: m.created_at, updated_at: m.updated_at, deleted: m.deleted } } }

#[derive(Debug)] #[tx_comp]
pub struct ToastyFileRepository { pub toasty: Arc<ToastyPlugin> }

#[async_trait]
impl FileRepository for ToastyFileRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<File>, anyhow::Error> { let mut db = self.toasty.db().clone(); match FileModel::get_by_id(&mut db, id).await { Ok(m) => Ok(Some(File::from(m))), Err(_) => Ok(None) } }
    async fn find_page(&self, page: u64, page_size: u64) -> Result<(Vec<File>, u64), anyhow::Error> { let mut db = self.toasty.db().clone(); let offset = (page - 1) * page_size; let total = FileModel::all().count().exec(&mut db).await? as u64; let models = FileModel::all().offset(offset as usize).limit(page_size as usize).exec(&mut db).await?; Ok((models.into_iter().filter(|m| m.deleted == 0).map(File::from).collect(), total)) }
    async fn save(&self, file: &File) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); if file.id == 0 { toasty::create!(FileModel { config_id: file.config_id.map(|v| v as i64).unwrap_or_default(), name: file.name.clone().unwrap_or_default(), file_path: file.file_path.clone(), url: file.url.clone(), file_type: file.file_type.clone().unwrap_or_default(), size: file.size, creator: file.creator.clone().unwrap_or_default(), updater: file.updater.clone().unwrap_or_default() }).exec(&mut db).await?; } else { let mut m = FileModel::get_by_id(&mut db, file.id).await.map_err(|_| anyhow::anyhow!("not found"))?; m.config_id = file.config_id.map(|v| v as i64).unwrap_or_default(); m.name = file.name.clone().unwrap_or_default(); m.file_path = file.file_path.clone(); m.url = file.url.clone(); m.file_type = file.file_type.clone().unwrap_or_default(); m.size = file.size; m.creator = file.creator.clone().unwrap_or_default(); m.updater = file.updater.clone().unwrap_or_default(); m.update().exec(&mut db).await?; } Ok(()) }
    async fn delete(&self, id: u64) -> Result<(), anyhow::Error> { let mut db = self.toasty.db().clone(); match FileModel::get_by_id(&mut db, id).await { Ok(mut m) => { m.deleted=1;m.update().exec(&mut db).await?; } Err(_) => {} } Ok(()) }
}
