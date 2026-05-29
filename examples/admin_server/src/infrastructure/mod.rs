/// 基础设施层模块
///
/// 提供仓储和缓存的具体实现。

pub mod persistence;
pub mod cache;

pub use persistence::{
    InMemoryUserRepository, InMemoryRoleRepository,
    InMemoryPermissionRepository, InMemoryTenantRepository,
    SeedDataService,
};
