//! gRPC 错误映射工具
//!
//! 将业务错误（`AppError`）精确映射为 `tonic::Status`。
//!
//! 基于 `AppError` 的结构化信息（`domain` + `code`）映射，**不依赖字符串匹配**：
//! - 内部错误（IO / 第三方库 / 未知）→ `Internal`（不外泄内部细节）
//! - `AUTH` / `SA` 等认证域 → `Unauthenticated`
//! - `REPOSITORY` 域按错误码区间：`101xx` 不存在 → `NotFound`，`100xx` 数据库 → `Internal`，
//!   `3xxxx` 重复 → `AlreadyExists`，`4xxxx` 校验 → `InvalidArgument`（防御性保留）
//! - 其他业务域 → 静态消息前缀兜底（消息为枚举静态值，非用户输入拼接）

use tonic::Status;
use tx_error::AppError;

/// 认证 / 授权相关 domain
const AUTH_DOMAINS: &[&str] = &["AUTH", "SA", "LOGIN"];

/// 将 `AppError` 映射为 `tonic::Status`
///
/// 按值接收（`AppError` 非 `Clone`），调用方直接 `map_err(err::to_status)` 即可。
pub fn to_status(e: AppError) -> Status {
    // 内部错误（IO / 第三方库 / 未识别）→ Internal，避免泄漏内部细节
    if e.is_internal() {
        tracing::error!("gRPC internal error: {}", e.full_message());
        return Status::internal("internal error");
    }

    let domain = e.domain();
    let code = e.code();

    // 认证 / 授权域 → Unauthenticated
    if AUTH_DOMAINS.contains(&domain) {
        return Status::unauthenticated(e.full_message());
    }

    // REPOSITORY 域：按错误码区间精确映射
    if domain == "REPOSITORY" {
        return match code {
            // 记录不存在
            10100..=10199 => Status::not_found(e.full_message()),
            // 记录已存在 / 重复
            30000..=39999 => Status::already_exists(e.full_message()),
            // 参数 / 业务校验失败
            40000..=49999 => Status::invalid_argument(e.full_message()),
            // 数据库异常
            10000..=10099 => {
                tracing::error!("gRPC database error: {}", e.full_message());
                Status::internal("database error")
            }
            _ => Status::internal(e.full_message()),
        };
    }

    // 其他业务域：静态消息前缀兜底（枚举静态消息，非用户输入）
    let msg = e.full_message();
    let lower = msg.to_lowercase();
    if lower.contains("权限") || lower.contains("permission") || lower.contains("forbidden") {
        Status::permission_denied(msg)
    } else if lower.contains("不存在") || lower.contains("not found") {
        Status::not_found(msg)
    } else if lower.contains("已存在") || lower.contains("already exists") {
        Status::already_exists(msg)
    } else {
        Status::internal(msg)
    }
}
