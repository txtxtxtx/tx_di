//! 持久化层
//!
//! 提供基于 toasty 0.6 ORM 的仓储实现。
//! 每个仓储通过 `#[tx_comp]` 注入 `ToastyPlugin` 获取 `Db` 实例。

mod user_repo;
mod role_repo;
mod menu_repo;
mod tenant_repo;
mod dept_repo;
mod post_repo;
mod dict_repo;
mod notice_repo;
mod notify_repo;
mod mail_repo;
mod oauth2_repo;
mod config_repo;
mod file_repo;
mod log_repo;
mod user_role_repo;
mod role_menu_repo;
mod user_post_repo;
mod seed_data;

pub use user_repo::ToastyUserRepository;
pub use role_repo::ToastyRoleRepository;
pub use menu_repo::ToastyMenuRepository;
pub use tenant_repo::ToastyTenantRepository;
pub use dept_repo::ToastyDeptRepository;
pub use post_repo::ToastyPostRepository;
pub use dict_repo::ToastyDictRepository;
pub use notice_repo::ToastyNoticeRepository;
pub use notify_repo::ToastyNotifyRepository;
pub use mail_repo::ToastyMailRepository;
pub use oauth2_repo::ToastyOAuth2Repository;
pub use config_repo::ToastyConfigRepository;
pub use file_repo::ToastyFileRepository;
pub use log_repo::ToastyLogRepository;
pub use user_role_repo::ToastyUserRoleRepository;
pub use role_menu_repo::ToastyRoleMenuRepository;
pub use user_post_repo::ToastyUserPostRepository;
pub use seed_data::SeedDataService;
