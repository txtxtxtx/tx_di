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

/// gRPC 服务反射所需的文件描述符集合（由 build.rs 生成）。
///
/// `tonic_reflection` 依赖此字节集向客户端暴露服务与方法定义，
/// 供 `grpcurl` / `grpcui` 等工具动态发现接口。
pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("pb/admin_descriptor.bin");

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
    GetUserInfoRequest, LoginRequest, LoginResponse, LogoutRequest, UserInfoResponse,
};
// --- User ---
pub use admin::user::{
    AssignDeptsRequest, AssignRolesRequest, ChangePasswordRequest, ChangeUserStatusRequest,
    CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, ListUsersResponse,
    UpdateUserRequest, UserIdRequest, UserResponse,
};
// --- Role ---
pub use admin::role::{
    AddUsersToRoleRequest, AssignMenusRequest, CreateRoleRequest, DeleteRoleRequest,
    GetRoleRequest, GetRoleUsersRequest, GetRoleUsersResponse, ListRolesRequest, ListRolesResponse,
    RemoveUsersFromRoleRequest, RoleResponse, UpdateRoleRequest,
};
// --- Menu ---
pub use admin::menu::{
    CreateMenuRequest, DeleteMenuRequest, GetMenuRequest, ListMenusRequest, ListMenusResponse,
    MenuResponse, UpdateMenuRequest,
};
// --- Department ---
pub use admin::dept::{
    CreateDeptRequest, DeleteDeptRequest, DeptResponse, GetDeptRequest, ListDeptsRequest,
    ListDeptsResponse, UpdateDeptRequest,
};
// --- Config ---
pub use admin::config::{
    ConfigResponse, CreateConfigRequest, DeleteConfigRequest, GetByKeysRequest, GetByKeysResponse,
    GetConfigRequest, ListConfigsRequest, ListConfigsResponse, UpdateConfigRequest,
};
// --- Dictionary ---
pub use admin::dict::{
    CreateDictDataRequest, CreateDictTypeRequest, DeleteDictDataRequest, DeleteDictTypeRequest,
    DictDataResponse, DictTypeResponse, GetByDictTypesRequest, GetByDictTypesResponse,
    GetDictDataRequest, GetDictTypeRequest, ListDictDataRequest, ListDictDataResponse,
    ListDictTypesRequest, ListDictTypesResponse, UpdateDictDataRequest, UpdateDictTypeRequest,
};
// --- Log ---
pub use admin::log::{
    CreateLoginLogRequest, CreateOperateLogRequest, DeleteLogsRequest, ListLoginLogsRequest,
    ListLoginLogsResponse, ListOperateLogsRequest, ListOperateLogsResponse, LoginLogResponse,
    OperateLogResponse,
};
// --- File ---
pub use admin::file::{
    CreateFileConfigRequest,
    DeleteFileConfigRequest,
    DeleteFileRequest,
    DownloadFileRequest,
    DownloadFileResponse,
    // 文件配置
    FileConfigResponse,
    FileResponse,
    GetFileConfigRequest,
    GetFileRequest,
    ListFileConfigsResponse,
    ListFilesRequest,
    ListFilesResponse,
    SetMasterFileConfigRequest,
    UpdateFileConfigRequest,
    UploadFileRequest,
};
// --- Job ---
pub use admin::job::{
    ChangeJobStatusRequest, CleanJobLogsRequest, CreateJobRequest, DeleteJobRequest,
    GetJobLogRequest, GetJobRequest, JobLogResponse, JobResponse, ListJobLogsRequest,
    ListJobLogsResponse, ListJobsRequest, ListJobsResponse, RunJobRequest, UpdateJobRequest,
};
// --- Monitor ---
pub use admin::monitor::{DiskInfo, NetworkInfo, OnlineUser, OnlineUserListResponse, ServerInfo};
// --- Tool ---
pub use admin::tool::{CacheInfo, CacheStatsResponse};

#[cfg(test)]
mod tests {
    //! DTO 序列化契约回归测试（G-6）
    //!
    //! 验证 HTTP/JSON 与 gRPC 共享 DTO 的关键序列化契约，防止 DTO 变更
    //! 破坏协议一致性（前端 JS 数值精度、camelCase 字段名、数字/字符串兼容）。

    use super::*;

    #[test]
    fn login_response_u64_serialized_as_string() {
        // u64 字段（user_id/tenant_id）必须序列化为字符串，避免 JS 精度丢失
        let resp = admin::auth::LoginResponse {
            user_id: 9007199254740993, // 超过 2^53，若按数字输出将丢失精度
            username: "admin".into(),
            nickname: "管理员".into(),
            tenant_id: 1,
            role_ids: vec![1, 2],
            permissions: vec!["system:user:list".into()],
            dept_ids: vec![3],
            token: "tok-123".into(),
            role_codes: vec!["admin".into()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(
            json.contains("\"userId\":\"9007199254740993\""),
            "userId 应序列化为字符串，实际: {json}"
        );
        assert!(
            json.contains("\"roleIds\":[\"1\",\"2\"]"),
            "roleIds 应序列化为字符串数组: {json}"
        );
        assert!(
            json.contains("\"tenantId\":\"1\""),
            "tenantId 应为字符串: {json}"
        );
    }

    #[test]
    fn login_response_field_names_camel_case() {
        // 字段名必须是 camelCase（前端契约）
        let resp = admin::auth::LoginResponse {
            user_id: 1,
            username: "admin".into(),
            nickname: "管理员".into(),
            tenant_id: 1,
            role_ids: vec![],
            permissions: vec![],
            dept_ids: vec![],
            token: "t".into(),
            role_codes: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        for key in ["userId", "tenantId", "roleIds", "deptIds", "roleCodes"] {
            assert!(
                json.contains(&format!("\"{key}\"")),
                "缺少 camelCase 字段 {key}: {json}"
            );
        }
        assert!(
            !json.contains("user_id"),
            "不应出现 snake_case 字段: {json}"
        );
    }

    #[test]
    fn page_request_accepts_number_or_string() {
        // 反序列化契约：page/size 同时接受数字与字符串
        let from_number: admin::common::PageRequest =
            serde_json::from_str(r#"{"page":1,"size":10}"#).unwrap();
        let from_string: admin::common::PageRequest =
            serde_json::from_str(r#"{"page":"2","size":"20"}"#).unwrap();
        assert_eq!(from_number.page, 1);
        assert_eq!(from_number.size, 10);
        assert_eq!(from_string.page, 2);
        assert_eq!(from_string.size, 20);

        // 序列化契约：始终输出字符串
        let json = serde_json::to_string(&from_number).unwrap();
        assert!(
            json.contains("\"page\":\"1\""),
            "page 应序列化为字符串: {json}"
        );
    }

    #[test]
    fn create_config_request_roundtrip() {
        // 配置请求：创建后序列化再反序列化，字段保持完整
        let req = admin::config::CreateConfigRequest {
            category: "system".into(),
            config_type: 0,
            name: "系统名称".into(),
            config_key: "sys.name".into(),
            value: "Admin".into(),
            remark: Some("备注".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: admin::config::CreateConfigRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.config_key, "sys.name");
        assert_eq!(back.value, "Admin");
        assert_eq!(back.remark.as_deref(), Some("备注"));
    }

    #[test]
    fn empty_message_serializes_to_empty_object() {
        // Empty 消息序列化后应为空对象（gRPC/HTTP 通用）
        let json = serde_json::to_string(&admin::common::Empty {}).unwrap();
        assert_eq!(json, "{}");
    }
}
