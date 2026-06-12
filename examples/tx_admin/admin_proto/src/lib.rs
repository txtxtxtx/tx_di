//! admin_proto - 基于 Protocol Buffers 的共享传输对象
//!
//! 由 tonic-build 从 `protos/` 目录生成，gRPC 和 HTTP 共用。
//! 所有 DTO 均由此 crate 统一生成，app 层和 api 层引用。
//!
//! 模块结构需与 prost 生成的跨包引用路径匹配：
//! `admin::auth` 中 `super::common` 指向 `admin::common`

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

    /// 权限
    pub mod permission {
        include!("pb/admin.permission.rs");
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
    CreateUserRequest, UpdateUserRequest, DeleteUserRequest, GetUserRequest,
    ListUsersRequest, ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest,
    UserResponse, ListUsersResponse,
};
// --- Role ---
pub use admin::role::{
    CreateRoleRequest, UpdateRoleRequest, DeleteRoleRequest, GetRoleRequest,
    ListRolesRequest, AssignMenusRequest, RoleResponse, ListRolesResponse,
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
// --- Permission ---
pub use admin::permission::{
    PermissionCheckRequest, PermissionCheckResponse,
    GetUserPermissionsRequest, UserPermissionsResponse, PermissionItem,
};
// --- Config ---
pub use admin::config::{
    CreateConfigRequest, UpdateConfigRequest, DeleteConfigRequest, GetConfigRequest,
    ListConfigsRequest, ConfigResponse, ListConfigsResponse,
};
// --- Dictionary ---
pub use admin::dict::{
    CreateDictTypeRequest, UpdateDictTypeRequest, DeleteDictTypeRequest, GetDictTypeRequest,
    ListDictTypesRequest, DictTypeResponse, ListDictTypesResponse,
    CreateDictDataRequest, UpdateDictDataRequest, DeleteDictDataRequest, GetDictDataRequest,
    ListDictDataRequest, DictDataResponse, ListDictDataResponse,
};
// --- Log ---
pub use admin::log::{
    CreateOperateLogRequest, ListOperateLogsRequest, OperateLogResponse, ListOperateLogsResponse,
    CreateLoginLogRequest, ListLoginLogsRequest, LoginLogResponse, ListLoginLogsResponse,
};
// --- File ---
pub use admin::file::{
    UploadFileRequest, DeleteFileRequest, GetFileRequest, ListFilesRequest,
    FileResponse, ListFilesResponse,
};

// ============================================================
// serde u64 辅助模块：uint64 <-> JSON string
// ============================================================

/// proton 中 uint64 在 JSON 中序列化为字符串，避免 JS 精度丢失
pub mod serde_u64 {
    use serde::{Deserialize, Serializer, Deserializer};

    pub fn serialize<S: Serializer>(val: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&val.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<u64>().map_err(serde::de::Error::custom)
    }
}
