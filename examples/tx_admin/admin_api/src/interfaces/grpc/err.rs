//! gRPC 错误映射工具
//!
//! 将业务错误（AppError）转换为 tonic::Status。

use tonic::Status;

/// 将 AppError 转换为 tonic::Status
///
/// 根据错误类型映射为合适的 gRPC 状态码：
/// - SaToken 认证错误 → Unauthenticated
/// - "not found" 类错误 → NotFound
/// - "already exists" 类错误 → AlreadyExists
/// - "invalid" / "权限" 类错误 → PermissionDenied
/// - 其他 → Internal
pub fn to_status(e: impl std::fmt::Display) -> Status {
    let msg = e.to_string();
    let msg_lower = msg.to_lowercase();

    // 认证错误
    if msg_lower.contains("token")
        || msg_lower.contains("unauthorized")
        || msg_lower.contains("未登录")
        || msg_lower.contains("认证")
    {
        return Status::unauthenticated(msg);
    }

    // 权限错误
    if msg_lower.contains("permission")
        || msg_lower.contains("权限")
        || msg_lower.contains("forbidden")
    {
        return Status::permission_denied(msg);
    }

    // 资源不存在
    if msg_lower.contains("not found") || msg_lower.contains("不存在") {
        return Status::not_found(msg);
    }

    // 资源已存在
    if msg_lower.contains("already exists") || msg_lower.contains("已存在") {
        return Status::already_exists(msg);
    }

    // 参数无效
    if msg_lower.contains("invalid") || msg_lower.contains("无效") {
        return Status::invalid_argument(msg);
    }

    Status::internal(msg)
}
