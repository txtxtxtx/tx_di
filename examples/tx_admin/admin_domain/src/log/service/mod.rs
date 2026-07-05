use std::sync::Arc;

use crate::log::model::aggregate::{LoginLog, OperateLog};
use crate::log::model::value_object::{LoginLogQuery, OperateLogQuery};
use crate::log::repository::{LoginLogRepository, OperateLogRepository};
use tx_common::page::Page;
use tx_di_core::{Component, DepsTuple};
use tx_error::AppResult;
use tx_common::id;

#[derive(Component)]
pub struct OperateLogService {
    log_repo: Arc<dyn OperateLogRepository>,
}

impl OperateLogService {
    /// 构造函数，创建操作日志服务实例
    ///
    /// # 参数
    /// * `log_repo` - 操作日志仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(log_repo: Arc<dyn OperateLogRepository>) -> Self {
        Self { log_repo }
    }

    /// 创建新的操作日志记录
    ///
    /// # 参数
    /// * `trace_id` - 链路追踪 ID，用于关联一次请求的完整调用链
    /// * `user_id` - 操作用户 ID
    /// * `user_type` - 用户类型标识（如管理员、普通用户等）
    /// * `log_type` - 日志大类（如系统日志、业务日志等）
    /// * `sub_type` - 日志子类型，更细粒度的分类
    /// * `biz_id` - 关联的业务数据 ID
    /// * `action` - 操作动作描述（如"创建"、"修改"、"删除"等）
    /// * `success` - 操作结果标识（成功/失败）
    /// * `extra` - 扩展信息，存储额外的上下文数据
    ///
    /// # 执行逻辑
    /// 1. 生成唯一日志 ID
    /// 2. 调用 OperateLog 聚合根的 create 方法构造日志实体
    /// 3. 将日志持久化到仓储
    ///
    /// # 返回
    /// 成功返回新创建的 OperateLog 聚合根
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储插入失败时
    pub async fn create_log(
        &self,
        trace_id: String,
        user_id: u64,
        user_type: i32,
        log_type: String,
        sub_type: String,
        biz_id: u64,
        action: String,
        success: i32,
        extra: String,
    ) -> AppResult<OperateLog> {
        let log_id = id::next_id();
        let log = OperateLog::create(
            log_id, trace_id, user_id, user_type, log_type, sub_type, biz_id, action, success, extra,
        );
        self.log_repo.insert(&log).await?;
        Ok(log)
    }

    /// 分页查询操作日志列表
    ///
    /// # 参数
    /// * `query` - 查询条件对象，包含用户ID、日志类型、时间范围等筛选字段
    /// * `page` - 分页参数，包含页码、每页条数等信息
    ///
    /// # 执行逻辑
    /// 1. 将查询条件和分页参数传递给仓储层
    /// 2. 仓储层执行分页查询并返回结果
    ///
    /// # 返回
    /// 成功返回包含操作日志列表的分页对象 Page<OperateLog>
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_log_page(
        &self,
        query: &OperateLogQuery,
        page: Page<OperateLog>,
    ) -> AppResult<Page<OperateLog>> {
        self.log_repo.find_page(query, page).await
    }

    /// 批量删除操作日志（物理删除）
    ///
    /// # 参数
    /// * `ids` - 要删除的日志 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 将 ID 列表传递给仓储层
    /// 2. 仓储层根据 ID 列表批量删除日志记录
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储删除失败时
    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_repo.delete_by_ids(ids).await
    }

    /// 清空全部操作日志
    ///
    /// # 执行逻辑
    /// 1. 调用仓储层的 clean_all 方法清空所有操作日志记录
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储清空操作失败时
    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_repo.clean_all().await
    }
}

#[derive(Component)]
pub struct LoginLogService {
    log_repo: Arc<dyn LoginLogRepository>,
}

impl LoginLogService {
    /// 构造函数，创建登录日志服务实例
    ///
    /// # 参数
    /// * `log_repo` - 登录日志仓储的 Arc 智能指针，用于数据持久化操作
    pub fn new(log_repo: Arc<dyn LoginLogRepository>) -> Self {
        Self { log_repo }
    }

    /// 创建新的登录日志记录
    ///
    /// # 参数
    /// * `user_id` - 登录用户 ID
    /// * `user_type` - 用户类型标识（如管理员、普通用户等）
    /// * `username` - 登录用户名
    /// * `login_ip` - 登录 IP 地址
    /// * `login_type` - 登录方式（如账号密码、OAuth、短信验证码等）
    /// * `result` - 登录结果标识（成功/失败）
    ///
    /// # 执行逻辑
    /// 1. 生成唯一日志 ID
    /// 2. 调用 LoginLog 聚合根的 create 方法构造登录日志实体
    /// 3. 将登录日志持久化到仓储
    ///
    /// # 返回
    /// 成功返回新创建的 LoginLog 聚合根
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储插入失败时
    pub async fn create_log(
        &self,
        user_id: u64,
        user_type: i32,
        username: String,
        login_ip: String,
        login_type: String,
        result: i32,
    ) -> AppResult<LoginLog> {
        let log_id = id::next_id();
        let log = LoginLog::create(log_id, user_id, user_type, username, login_ip, login_type, result);
        self.log_repo.insert(&log).await?;
        Ok(log)
    }

    /// 分页查询登录日志列表
    ///
    /// # 参数
    /// * `query` - 查询条件对象，包含用户名、登录IP、登录时间范围等筛选字段
    /// * `page` - 分页参数，包含页码、每页条数等信息
    ///
    /// # 执行逻辑
    /// 1. 将查询条件和分页参数传递给仓储层
    /// 2. 仓储层执行分页查询并返回结果
    ///
    /// # 返回
    /// 成功返回包含登录日志列表的分页对象 Page<LoginLog>
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储查询失败时
    pub async fn get_log_page(
        &self,
        query: &LoginLogQuery,
        page: Page<LoginLog>,
    ) -> AppResult<Page<LoginLog>> {
        self.log_repo.find_page(query, page).await
    }

    /// 批量删除登录日志（物理删除）
    ///
    /// # 参数
    /// * `ids` - 要删除的日志 ID 列表
    ///
    /// # 执行逻辑
    /// 1. 将 ID 列表传递给仓储层
    /// 2. 仓储层根据 ID 列表批量删除日志记录
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储删除失败时
    pub async fn delete_logs(&self, ids: &[u64]) -> AppResult<()> {
        self.log_repo.delete_by_ids(ids).await
    }

    /// 清空全部登录日志
    ///
    /// # 执行逻辑
    /// 1. 调用仓储层的 clean_all 方法清空所有登录日志记录
    ///
    /// # 返回
    /// 成功返回 ()
    ///
    /// # 错误
    /// - 数据库操作错误 - 仓储清空操作失败时
    pub async fn clean_logs(&self) -> AppResult<()> {
        self.log_repo.clean_all().await
    }
}

#[cfg(test)]
mod tests;
