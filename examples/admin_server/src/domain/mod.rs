/// 领域层模块
///
/// 目录结构：
/// - `user`            — 用户聚合根
/// - `role`            — 角色聚合
/// - `menu`            — 菜单聚合
/// - `dept`            — 部门聚合
/// - `post`            — 岗位聚合
/// - `tenant`          — 租户聚合
/// - `dict`            — 字典聚合
/// - `login_log`       — 登录日志
/// - `operate_log`     — 操作日志
/// - `api_log`         — API日志
/// - `notice`          — 通知公告
/// - `notify`          — 站内信
/// - `mail`            — 邮件聚合
/// - `oauth2`          — OAuth2聚合
/// - `config`          — 配置聚合
/// - `file`            — 文件聚合
/// - `user_role`       — 用户角色关联
/// - `role_menu`       — 角色菜单关联
/// - `user_post`       — 用户岗位关联
/// - `data_permission` — 数据权限值对象与领域服务
/// - `permission`      — 兼容旧代码的 re-export

pub mod user;
pub mod role;
pub mod menu;
pub mod dept;
pub mod post;
pub mod tenant;
pub mod dict;
pub mod login_log;
pub mod operate_log;
pub mod api_log;
pub mod notice;
pub mod notify;
pub mod mail;
pub mod oauth2;
pub mod config;
pub mod file;
pub mod user_role;
pub mod role_menu;
pub mod user_post;
pub mod data_permission;
pub mod permission; // 兼容旧代码

// 常用类型重新导出
pub use user::{User, UserStatus, Sex, UserRepository};
pub use role::{Role, RoleStatus, RoleType, RoleRepository};
pub use menu::{Menu, MenuType, MenuStatus, MenuRepository};
pub use dept::{Dept, CommonStatus, DeptRepository};
pub use post::{Post, PostRepository};
pub use tenant::{Tenant, TenantStatus, TenantPackage, TenantRepository};
pub use dict::{DictType, DictData, DictRepository};
pub use login_log::{LoginLog, LoginLogType, LoginResult, LoginLogRepository};
pub use operate_log::{OperateLog, OperateLogRepository};
pub use api_log::{ApiAccessLog, ApiErrorLog, ApiLogRepository};
pub use notice::{Notice, NoticeType, NoticeRepository};
pub use notify::{NotifyMessage, NotifyTemplate, NotifyRepository};
pub use mail::{MailAccount, MailTemplate, MailLog, MailSendStatus, MailRepository};
pub use oauth2::{OAuth2Client, OAuth2AccessToken, OAuth2RefreshToken, OAuth2Repository};
pub use config::{Config, ConfigType, ConfigRepository};
pub use file::File;
pub use user_role::UserRole;
pub use role_menu::RoleMenu;
pub use user_post::UserPost;
pub use data_permission::{DataScope, DataPermissionContext, DataPermissionService};
