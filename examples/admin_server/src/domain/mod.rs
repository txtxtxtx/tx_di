/// 领域层
///
/// 每个聚合一个子目录：mod.rs（实体+trait）、repo.rs（toasty 实现）

pub mod error;
pub mod data_permission;
pub mod seed_data;

pub mod user;
pub mod role;
pub mod menu;
pub mod tenant;
pub mod dept;
pub mod post;
pub mod dict;
pub mod config;
pub mod file;
pub mod login_log;
pub mod operate_log;
pub mod api_log;
pub mod notice;
pub mod notify;
pub mod mail;
pub mod oauth2;

use std::fmt::Display;

// 重新导出常用类型
pub use error::{AdminErr, AdminResult};
pub use tx_error::AppError;
pub use user::{User, UserStatus, Sex, UserRepository, service::UserService};
pub use role::{Role, RoleStatus, RoleType, RoleRepository};
pub use menu::{Menu, MenuType, MenuStatus, MenuRepository};
pub use tenant::{Tenant, TenantStatus, TenantPackage, TenantRepository};
pub use dept::{Dept, CommonStatus, DeptRepository};
pub use post::{Post, PostRepository};
pub use dict::{DictType, DictData, DictRepository};
pub use config::{Config, ConfigType, ConfigRepository};
pub use file::{File, FileRepository};
pub use login_log::{LoginLog, LoginLogType, LoginResult, LoginLogRepository};
pub use operate_log::{OperateLog, OperateLogRepository};
pub use api_log::{ApiAccessLog, ApiErrorLog, ApiLogRepository};
pub use notice::{Notice, NoticeType, NoticeRepository};
pub use notify::{NotifyMessage, NotifyTemplate, NotifyRepository};
pub use mail::{MailAccount, MailTemplate, MailLog, MailSendStatus, MailRepository};
pub use oauth2::{OAuth2Client, OAuth2AccessToken, OAuth2RefreshToken, OAuth2Repository};
pub use data_permission::{DataScope, DataPermissionContext, DataPermissionService};


#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum DeletedStatus {
    #[column(variant = 0_u8)]
    Normal,
    #[column(variant = 1_u8)]
    Deleted,
}
impl Display for DeletedStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeletedStatus::Normal => write!(f, "normal"),
            DeletedStatus::Deleted => write!(f, "deleted"),
        }
    }
}