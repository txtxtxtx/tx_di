use std::sync::Arc;

use crate::file::model::aggregate::{File, FileConfig};
use crate::file::model::value_object::{FileDownloadInfo, FileQuery, FileUploadCommand};
use crate::file::repository::{FileConfigRepository, FileRepository};
use crate::shared::repository::RepositoryError;
use tx_common::page::Page;
use tx_di_core::tx_comp;
use tx_error::AppResult;
use tx_common::id;

#[tx_comp]
pub struct FileService {
    file_repo: Arc<dyn FileRepository>,
    file_config_repo: Arc<dyn FileConfigRepository>,
}

impl FileService {
    /// 构造函数，创建文件服务实例
    ///
    /// # 参数
    /// * `file_repo` - 文件仓储的 Arc 智能指针，用于文件元数据的持久化操作
    /// * `file_config_repo` - 文件配置仓储的 Arc 智能指针，用于存储策略配置
    pub fn new(
        file_repo: Arc<dyn FileRepository>,
        file_config_repo: Arc<dyn FileConfigRepository>,
    ) -> Self {
        Self {
            file_repo,
            file_config_repo,
        }
    }

    /// 上传文件，记录文件元数据
    ///
    /// # 参数
    /// * `cmd` - 文件上传命令对象，包含文件名、存储路径、访问URL、文件类型、文件大小等信息
    /// * `creator` - 创建者标识，可选
    ///
    /// # 执行逻辑
    /// 1. 生成唯一文件 ID
    /// 2. 调用 File 聚合根的 create 方法构造文件实体
    /// 3. 将文件元数据持久化到仓储
    ///
    /// # 返回
    /// 成功返回新创建的 File 聚合根
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储插入失败时
    pub async fn upload_file(
        &self,
        cmd: FileUploadCommand,
        creator: Option<String>,
    ) -> AppResult<File> {
        let file_id = id::next_id();
        let file = File::create(
            file_id,
            cmd.config_id,
            cmd.name,
            cmd.path,
            cmd.url,
            cmd.file_type,
            cmd.size,
            creator,
        );
        self.file_repo.insert(&file).await?;
        Ok(file)
    }

    /// 软删除文件（逻辑删除）
    ///
    /// # 参数
    /// * `file_id` - 要删除的文件 ID
    /// * `updater` - 操作者标识，可选，用于记录删除操作人
    ///
    /// # 执行逻辑
    /// 1. 根据 file_id 查找文件记录，若不存在则返回未找到错误
    /// 2. 调用聚合根的 soft_delete 方法标记为已删除
    /// 3. 将状态变更持久化到仓储
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - `NotFoundFile` - 当指定 file_id 的文件不存在时
    /// - 数据库操作错误 - 仓储查询或更新失败时
    pub async fn delete_file(&self, file_id: u64, updater: Option<String>) -> AppResult<()> {
        let mut file = self
            .file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?;

        file.soft_delete(updater);
        self.file_repo.update(&file).await?;
        Ok(())
    }

    /// 分页查询文件列表
    ///
    /// # 参数
    /// * `query` - 查询条件对象，包含文件名、文件类型等筛选字段
    /// * `page` - 分页参数，包含页码、每页条数等信息
    ///
    /// # 执行逻辑
    /// 1. 将查询条件和分页参数传递给仓储层
    /// 2. 仓储层执行分页查询并返回结果
    ///
    /// # 返回
    /// 成功返回包含文件列表的分页对象 Page<File>
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_file_page(
        &self,
        query: &FileQuery,
        page: Page<File>,
    ) -> AppResult<Page<File>> {
        self.file_repo.find_page(query, page).await
    }

    /// 根据 ID 获取单个文件详情
    ///
    /// # 参数
    /// * `file_id` - 文件 ID
    ///
    /// # 执行逻辑
    /// 1. 根据 file_id 从仓储中查找文件记录
    /// 2. 若找到则返回，若未找到则返回未找到错误
    ///
    /// # 返回
    /// 成功返回对应的 File 聚合根
    ///
    /// # 错误
    /// - `NotFoundFile` - 当指定 file_id 的文件不存在时
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_file(&self, file_id: u64) -> AppResult<File> {
        Ok(self.file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?)
    }

    /// 获取文件下载信息
    ///
    /// # 参数
    /// * `file_id` - 文件 ID
    ///
    /// # 执行逻辑
    /// 1. 根据 file_id 从仓储中查找文件记录，若不存在则返回未找到错误
    /// 2. 根据文件扩展名自动推断 MIME 类型（Content-Type）
    /// 3. 支持的文件类型包括：pdf、jpg/jpeg、png、gif、txt、html、css、js、json、xml、zip、doc/docx、xls/xlsx
    /// 4. 未知扩展名默认使用 application/octet-stream
    /// 5. 构建 FileDownloadInfo 对象返回
    ///
    /// # 返回
    /// 成功返回 FileDownloadInfo，包含文件URL、文件名、文件大小和Content-Type
    ///
    /// # 错误
    /// - `NotFoundFile` - 当指定 file_id 的文件不存在时
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn download_file(&self, file_id: u64) -> AppResult<FileDownloadInfo> {
        let file = self.file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?;

        // Determine MIME type from file extension
        let content_type = match file.name.rsplit('.').next() {
            Some("pdf") => "application/pdf",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("txt") => "text/plain",
            Some("html" | "htm") => "text/html",
            Some("css") => "text/css",
            Some("js") => "application/javascript",
            Some("json") => "application/json",
            Some("xml") => "application/xml",
            Some("zip") => "application/zip",
            Some("doc" | "docx") => "application/msword",
            Some("xls" | "xlsx") => "application/vnd.ms-excel",
            _ => "application/octet-stream",
        };

        Ok(FileDownloadInfo {
            url: file.url,
            filename: file.name,
            size: file.size,
            content_type: content_type.to_string(),
            storage_path: file.path,
        })
    }

    // ========================================================================
    // 配置 ID 解析 & 校验
    // ========================================================================

    /// 解析配置 ID：显式指定则直接使用，否则回退到主配置。
    ///
    /// 业务规则："未指定存储配置时默认使用主配置"。
    pub async fn resolve_config_id(&self, config_id: Option<i32>) -> AppResult<Option<i32>> {
        match config_id {
            Some(id) => Ok(Some(id)),
            None => Ok(self.file_config_repo.find_master().await?.map(|c| c.id)),
        }
    }

    /// 从 DB 文件配置中读取允许的文件扩展名列表（不含插件 TOML 回退）。
    ///
    /// 业务规则由 app 层组合：DB 白名单 + 插件默认白名单 → 合并校验。
    pub async fn get_allowed_extensions(&self, config_id: Option<i32>) -> AppResult<Vec<String>> {
        let db_config = if let Some(cid) = config_id {
            self.file_config_repo.find_by_id(cid).await?
        } else {
            self.file_config_repo.find_master().await?
        };

        Ok(db_config
            .and_then(|cfg| {
                serde_json::from_str::<serde_json::Value>(&cfg.config)
                    .ok()
                    .and_then(|json| {
                        json.get("allowed_extensions")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|s| s.as_str().map(String::from))
                                    .collect()
                            })
                    })
            })
            .unwrap_or_default())
    }

    // ========================================================================
    // 文件配置 CRUD
    // ========================================================================

    /// 获取全部配置列表
    pub async fn get_config_all(&self) -> AppResult<Vec<FileConfig>> {
        self.file_config_repo.find_all().await
    }

    /// 按 ID 获取配置
    pub async fn get_config(&self, id: i32) -> AppResult<FileConfig> {
        Ok(self
            .file_config_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?)
    }

    /// 创建配置
    pub async fn create_config(
        &self,
        name: String,
        storage: i32,
        remark: Option<String>,
        config: String,
        creator: Option<String>,
    ) -> AppResult<FileConfig> {
        let id = (jiff::Timestamp::now().as_millisecond() % i32::MAX as i64) as i32;
        let agg = FileConfig::create(id, name, storage, remark, config, creator);
        self.file_config_repo.insert(&agg).await?;
        Ok(agg)
    }

    /// 更新配置
    pub async fn update_config(
        &self,
        id: i32,
        name: String,
        storage: i32,
        remark: Option<String>,
        config: String,
        updater: Option<String>,
    ) -> AppResult<FileConfig> {
        let mut agg = self
            .file_config_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?;
        agg.update_info(name, storage, remark, config, updater);
        self.file_config_repo.update(&agg).await?;
        Ok(agg)
    }

    /// 软删除配置
    pub async fn delete_config(&self, id: i32, updater: Option<String>) -> AppResult<()> {
        let mut agg = self
            .file_config_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?;
        agg.soft_delete(updater);
        self.file_config_repo.update(&agg).await?;
        Ok(())
    }

    /// 设为主配置（保证唯一主配置不变式）
    ///
    /// 业务规则：同一时刻最多存在一个主配置。切换时先取消当前主配置，
    /// 再设置新主配置。
    pub async fn set_master_config(
        &self,
        id: i32,
        updater: Option<String>,
    ) -> AppResult<FileConfig> {
        // 1. 取消当前主配置
        if let Some(mut current_master) = self.file_config_repo.find_master().await? {
            if current_master.id != id {
                current_master.unset_master(updater.clone());
                self.file_config_repo.update(&current_master).await?;
            }
        }

        // 2. 设置新主配置
        let mut agg = self
            .file_config_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepositoryError::NotFoundFile)?;
        agg.set_master(updater);
        self.file_config_repo.update(&agg).await?;
        Ok(agg)
    }
}

#[cfg(test)]
mod tests;
