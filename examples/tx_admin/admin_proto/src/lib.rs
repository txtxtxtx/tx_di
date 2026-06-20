//! admin_proto - 基于 Protocol Buffers 的共享传输对象
//!
//! 由 tonic-build 从 `protos/` 目录生成，gRPC 和 HTTP 共用。
//! 所有 DTO 均由此 crate 统一生成，app 层和 api 层引用。
//!
//! 模块结构需与 prost 生成的跨包引用路径匹配：
//! `admin::auth` 中 `super::common` 指向 `admin::common`
//!
//! 所有 i64/u64 字段通过 serde_with 的 DisplayFromStr 序列化为 JSON 字符串，
//! 避免 JavaScript 数值精度丢失。

pub mod flexible_serde;

pub mod admin {
    /// 通用类型（PageRequest, Empty, PageResponse 等）
    pub mod common {
        include!("pb/admin.common.rs");
    }

    /// 认证
    pub mod auth {
        include!("pb/admin.auth.rs");
    }

    /// 用户
    pub mod user {
        include!("pb/admin.user.rs");
    }

    /// 角色
    pub mod role {
        include!("pb/admin.role.rs");
    }

    /// 菜单
    pub mod menu {
        include!("pb/admin.menu.rs");
    }

    /// 部门
    pub mod dept {
        include!("pb/admin.dept.rs");
    }

    /// 配置
    pub mod config {
        include!("pb/admin.config.rs");
    }

    /// 字典
    pub mod dict {
        include!("pb/admin.dict.rs");
    }

    /// 日志
    pub mod log {
        include!("pb/admin.log.rs");
    }

    /// 文件
    pub mod file {
        include!("pb/admin.file.rs");
    }

    /// 定时任务
    pub mod job {
        include!("pb/admin.job.rs");
    }

    /// 系统监控
    pub mod monitor {
        include!("pb/admin.monitor.rs");
    }

    /// 系统工具
    pub mod tool {
        include!("pb/admin.tool.rs");
    }
}

// ============================================================
// 公开快捷导出
// ============================================================

// --- Common ---
pub use admin::common::{Empty, PageRequest, PageResponse};

// --- Auth ---
pub use admin::auth::{
    LoginRequest, LoginResponse, GetUserInfoRequest, UserInfoResponse, LogoutRequest,
};
// --- User ---
pub use admin::user::{
    CreateUserRequest, UpdateUserRequest, ChangeUserStatusRequest, DeleteUserRequest, GetUserRequest,
    ListUsersRequest, ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest,
    UserResponse, ListUsersResponse, UserIdRequest,
};
// --- Role ---
pub use admin::role::{
    CreateRoleRequest, UpdateRoleRequest, DeleteRoleRequest, GetRoleRequest,
    ListRolesRequest, AssignMenusRequest, RoleResponse, ListRolesResponse,
    GetRoleUsersRequest, GetRoleUsersResponse, AddUsersToRoleRequest, RemoveUsersFromRoleRequest,
};
// --- Menu ---
pub use admin::menu::{
    CreateMenuRequest, UpdateMenuRequest, DeleteMenuRequest, GetMenuRequest,
    ListMenusRequest, MenuResponse, ListMenusResponse,
};
// --- Department ---
pub use admin::dept::{
    CreateDeptRequest, UpdateDeptRequest, DeleteDeptRequest, GetDeptRequest,
    ListDeptsRequest, DeptResponse, ListDeptsResponse,
};
// --- Config ---
pub use admin::config::{
    CreateConfigRequest, UpdateConfigRequest, DeleteConfigRequest, GetConfigRequest,
    ListConfigsRequest, ConfigResponse, ListConfigsResponse,
    GetByKeysRequest, GetByKeysResponse,
};
// --- Dictionary ---
pub use admin::dict::{
    CreateDictTypeRequest, UpdateDictTypeRequest, DeleteDictTypeRequest, GetDictTypeRequest,
    ListDictTypesRequest, DictTypeResponse, ListDictTypesResponse,
    CreateDictDataRequest, UpdateDictDataRequest, DeleteDictDataRequest, GetDictDataRequest,
    ListDictDataRequest, DictDataResponse, ListDictDataResponse,
    GetByDictTypesRequest, GetByDictTypesResponse,
};
// --- Log ---
pub use admin::log::{
    CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse, ListOperateLogsResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse, ListLoginLogsResponse,
    DeleteLogsRequest,
};
// --- File ---
pub use admin::file::{
    UploadFileRequest, DeleteFileRequest, GetFileRequest, ListFilesRequest,
    FileResponse, ListFilesResponse,
    DownloadFileRequest, DownloadFileResponse,
    // 文件配置
    FileConfigResponse, ListFileConfigsResponse,
    GetFileConfigRequest, CreateFileConfigRequest, UpdateFileConfigRequest,
    DeleteFileConfigRequest, SetMasterFileConfigRequest,
};
// --- Job ---
pub use admin::job::{
    JobResponse, CreateJobRequest, UpdateJobRequest, DeleteJobRequest,
    GetJobRequest, ListJobsRequest, ListJobsResponse,
    ChangeJobStatusRequest, RunJobRequest,
    JobLogResponse, ListJobLogsRequest, ListJobLogsResponse,
    GetJobLogRequest, CleanJobLogsRequest,
};
// --- Monitor ---
pub use admin::monitor::{ServerInfo, DiskInfo, NetworkInfo, OnlineUser, OnlineUserListResponse};
// --- Tool ---
pub use admin::tool::{CacheInfo, CacheStatsResponse};
