//! 种子数据初始化
//!
//! 在应用启动时自动创建默认的超级管理员、角色、权限和租户数据。

use std::sync::Arc;
use tx_di_core::{tx_comp, CompInit, RIE, App, async_method};
use tokio_util::sync::CancellationToken;

use crate::domain::data_permission::DataScope;
use crate::domain::permission::{Permission, PermissionType};
use crate::domain::role::Role;
use crate::domain::tenant::Tenant;
use crate::domain::user::User;

use super::InMemoryUserRepository;
use super::InMemoryRoleRepository;
use super::InMemoryPermissionRepository;
use super::InMemoryTenantRepository;

/// 种子数据初始化服务
///
/// 在 `async_init` 阶段自动插入默认数据。
#[derive(Debug)]
#[tx_comp(init)]
pub struct SeedDataService {
    pub user_repo: Arc<InMemoryUserRepository>,
    pub role_repo: Arc<InMemoryRoleRepository>,
    pub perm_repo: Arc<InMemoryPermissionRepository>,
    pub tenant_repo: Arc<InMemoryTenantRepository>,
}

impl SeedDataService {
    /// 初始化种子数据
    async fn seed(&self) -> Result<(), anyhow::Error> {
        tracing::info!("正在初始化种子数据...");

        // ── 1. 创建默认租户 ──
        let tenant_id = 1;
        let tenant = Tenant::new(
            tenant_id.clone(),
            "默认租户".to_string(),
            "default".to_string(),
        );
        self.tenant_repo.save(&tenant).await?;

        // ── 2. 创建系统权限（菜单 + 按钮 + API） ──
        let perms = vec![
            // 系统管理
            self.make_perm("p-sys", "系统管理", "system", PermissionType::Directory, None, 1),
            // 用户管理
            self.make_perm("p-user", "用户管理", "system:user", PermissionType::Menu, Some("p-sys"), 10),
            self.make_perm("p-user-list", "用户查询", "system:user:list", PermissionType::Button, Some("p-user"), 11),
            self.make_perm("p-user-create", "用户新增", "system:user:create", PermissionType::Button, Some("p-user"), 12),
            self.make_perm("p-user-update", "用户修改", "system:user:update", PermissionType::Button, Some("p-user"), 13),
            self.make_perm("p-user-delete", "用户删除", "system:user:delete", PermissionType::Button, Some("p-user"), 14),
            // 角色管理
            self.make_perm("p-role", "角色管理", "system:role", PermissionType::Menu, Some("p-sys"), 20),
            self.make_perm("p-role-list", "角色查询", "system:role:list", PermissionType::Button, Some("p-role"), 21),
            self.make_perm("p-role-create", "角色新增", "system:role:create", PermissionType::Button, Some("p-role"), 22),
            // 权限管理
            self.make_perm("p-perm", "权限管理", "system:permission", PermissionType::Menu, Some("p-sys"), 30),
            // 租户管理
            self.make_perm("p-tenant", "租户管理", "system:tenant", PermissionType::Menu, Some("p-sys"), 40),
            self.make_perm("p-tenant-list", "租户查询", "system:tenant:list", PermissionType::Button, Some("p-tenant"), 41),
            // 文件管理
            self.make_perm("p-file", "文件管理", "system:file", PermissionType::Menu, Some("p-sys"), 50),
        ];

        for perm in &perms {
            self.perm_repo.save(perm).await?;
        }

        // ── 3. 创建超级管理员角色 ──
        let admin_role_id = "r-admin-001".to_string();
        let mut admin_role = Role::new(
            admin_role_id.clone(),
            tenant_id.clone(),
            "超级管理员".to_string(),
            "admin".to_string(),
            0,
        );
        admin_role.permission_ids = perms.iter().map(|p| p.id.clone()).collect();
        admin_role.data_scope = DataScope::All;
        self.role_repo.save(&admin_role).await?;

        // 创建普通角色
        let user_role_id = "r-user-001".to_string();
        let mut user_role = Role::new(
            user_role_id.clone(),
            tenant_id.clone(),
            "普通用户".to_string(),
            "user".to_string(),
            1,
        );
        user_role.permission_ids = vec![
            "p-user-list".to_string(),
            "p-role-list".to_string(),
            "p-file".to_string(),
        ];
        user_role.data_scope = DataScope::Self_;
        self.role_repo.save(&user_role).await?;

        // ── 4. 创建管理员用户 ──
        let admin_password = bcrypt::hash("admin123", bcrypt::DEFAULT_COST)?;
        let mut admin_user = User::new(
            "u-admin-001".to_string(),
            tenant_id.clone(),
            "admin".to_string(),
            admin_password,
            "系统管理员".to_string(),
        );
        admin_user.email = Some("admin@example.com".to_string());
        admin_user.assign_roles(vec![admin_role_id]);
        self.user_repo.save(&admin_user).await?;

        // 创建普通用户
        let user_password = bcrypt::hash("user123", bcrypt::DEFAULT_COST)?;
        let normal_user = User::new(
            "u-user-001".to_string(),
            tenant_id,
            "user".to_string(),
            user_password,
            "普通用户".to_string(),
        );
        self.user_repo
            .save(&{
                let mut u = normal_user;
                u.assign_roles(vec![user_role_id]);
                u
            })
            .await?;

        tracing::info!(
            "种子数据初始化完成: 1个租户, {}个权限, 2个角色, 2个用户",
            perms.len()
        );
        tracing::info!("管理员: admin / admin123");
        tracing::info!("普通用户: user / user123");

        Ok(())
    }

    fn make_perm(
        id: &str,
        name: &str,
        code: &str,
        perm_type: PermissionType,
        parent_id: Option<&str>,
        sort: i32,
    ) -> Permission {
        Permission::new(
            id.to_string(),
            name.to_string(),
            code.to_string(),
            perm_type,
            parent_id.map(|s| s.to_string()),
            sort,
        )
    }
}

impl CompInit for SeedDataService {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let service = ctx.inject::<SeedDataService>();
            service.seed().await?;
            Ok(())
        }
    );

    fn init_sort() -> i32 {
        // 在数据库初始化之后执行
        1000
    }
}
