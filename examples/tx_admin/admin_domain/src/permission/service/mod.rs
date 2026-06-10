use std::sync::Arc;  // 引入Arc智能指针，用于共享所有权
use tx_error::AppResult;  // 引入自定义错误类型AppResult，用于处理可能的错误
use crate::permission::model::value_object::PermissionCheck;  // 引入权限检查的值对象
use crate::permission::repository::PermissionRepository;  // 引入权限仓库trait

/// Permission domain service
/// 权限领域服务，负责处理权限相关的业务逻辑
pub struct PermissionService {
    permission_repo: Arc<dyn PermissionRepository>,  // 权限仓库的Arc包装，支持多线程共享
}

impl PermissionService {
    /// 创建新的PermissionService实例
    /// # 参数
    /// * `permission_repo` - 权限仓库的Arc包装
    /// # 返回值
    /// 返回一个新的PermissionService实例
    pub fn new(permission_repo: Arc<dyn PermissionRepository>) -> Self {
        Self { permission_repo }
    }

    /// 获取指定用户的所有权限
    ///
    /// 这是一个异步方法，用于从权限存储库中检索特定用户的所有权限。
    ///
    /// # 参数
    /// * `user_id` - 用户ID，类型为u64，用于标识要查询的用户
    ///
    /// # 返回值
    /// * `AppResult<Vec<String>>` - 返回一个AppResult，其中包含权限字符串的向量(Vec)，
    ///   表示用户拥有的所有权限。如果发生错误，则返回错误信息。
    ///
    /// # 异常
    /// * 如果在查询过程中发生错误，可能会返回AppResult的错误变体
    pub async fn get_user_permissions(
        &self,  // 引用self，表示这是对结构体实例的方法
        user_id: u64,  // 用户ID参数，类型为u64
    ) -> AppResult<Vec<String>> {  // 返回类型为AppResult，其中包含字符串向量
        self.permission_repo.find_by_user_id(user_id).await  // 调用权限存储库的异步方法，根据用户ID查找权限
    }

    /// Check if user has specific permission
    pub async fn check_permission(
        &self,
        user_id: u64,
        code: &str,
    ) -> AppResult<bool> {
        let permissions = self.permission_repo.find_by_user_id(user_id).await?;
        Ok(permissions.iter().any(|p| p == code))
    }

    /// Get permissions for role set
    pub async fn get_role_permissions(
        &self,
        role_ids: &[u64],
    ) -> AppResult<Vec<String>> {
        self.permission_repo.find_by_role_ids(role_ids).await
    }

    /// Get all available permissions
    pub async fn get_all_permissions(
        &self,
    ) -> AppResult<Vec<PermissionCheck>> {
        self.permission_repo.find_all().await
    }
}
