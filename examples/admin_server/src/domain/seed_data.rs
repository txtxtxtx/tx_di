//! 种子数据初始化

use std::sync::Arc;
use tx_di_core::{tx_comp, CompInit, RIE, App, async_method};
use tokio_util::sync::CancellationToken;

use super::data_permission::DataScope;
use super::menu::{Menu, MenuRepository};
use super::role::{Role, RoleRepository};
use super::tenant::{Tenant, TenantRepository};
use super::user::{User, UserRepository};
use super::user::repo::ToastyUserRepository;
use super::role::repo::ToastyRoleRepository;
use super::menu::repo::ToastyMenuRepository;
use super::tenant::repo::ToastyTenantRepository;

#[derive(Debug)]
#[tx_comp(init)]
pub struct SeedDataService {
    pub user_repo: Arc<ToastyUserRepository>,
    pub role_repo: Arc<ToastyRoleRepository>,
    pub perm_repo: Arc<ToastyMenuRepository>,
    pub tenant_repo: Arc<ToastyTenantRepository>,
}

impl SeedDataService {
    pub async fn seed(&self) -> Result<(), anyhow::Error> {
        tracing::info!("正在初始化种子数据...");
        let tenant = Tenant::new("默认租户".to_string());
        self.tenant_repo.save(&tenant).await?;
        let perms = vec![
            Menu::directory("系统管理".to_string(), 0, 1, None),
            Menu::menu("用户管理".to_string(), 0, 10, "/system/user".to_string(), "system/user/index".to_string(), None),
            Menu::menu("角色管理".to_string(), 0, 20, "/system/role".to_string(), "system/role/index".to_string(), None),
        ];
        for perm in &perms { self.perm_repo.save(perm).await?; }
        let mut admin_role = Role::new(1, "超级管理员".to_string(), "admin".to_string(), 0);
        admin_role.data_scope = DataScope::All;
        self.role_repo.save(&admin_role).await?;
        let admin_pw = bcrypt::hash("admin123", bcrypt::DEFAULT_COST)?;
        let admin = User::new(1, "admin".to_string(), admin_pw, "系统管理员".to_string());
        self.user_repo.save(&admin).await?;
        tracing::info!("种子数据初始化完成");
        Ok(())
    }
}

impl CompInit for SeedDataService {
    async_method!(fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> { let s = ctx.inject::<SeedDataService>(); s.seed().await?; Ok(()) });
    fn init_sort() -> i32 { 1000 }
}
