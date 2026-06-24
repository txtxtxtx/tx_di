//! gRPC 认证拦截器
//!
//! 从 gRPC metadata 提取 Bearer token，通过 sa-token 验证并注入 login_id。
//! 公开方法（如 Login）跳过认证。

use sa_token_core::token::TokenValue;
use tonic::{Request, Status};
use tracing::warn;
use tx_di_sa_token::StpUtil;

/// 请求扩展中存储 login_id 的 key
#[derive(Debug, Clone)]
pub struct GrpcLoginId(pub String);

/// 请求扩展中存储原始 token 的 key（用于后续 logout 等操作）
#[derive(Debug, Clone)]
pub struct GrpcToken(pub TokenValue);

/// 不需要认证的 gRPC 方法全名列表
const OPEN_METHODS: &[&str] = &[
    "/admin.auth.AuthService/Login",
];

/// gRPC 认证拦截器
///
/// 实现 tonic::service::Interceptor，从 metadata 提取 Bearer token 验证。
#[derive(Clone)]
pub struct GrpcAuthInterceptor;

impl GrpcAuthInterceptor {
    pub fn new() -> Self {
        Self
    }
}

impl tonic::service::Interceptor for GrpcAuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        // 获取请求方法名（gRPC 路径格式: /package.Service/Method）
        let method = req
            .metadata()
            .get("uri")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // 公开方法跳过认证
        if OPEN_METHODS.iter().any(|&m| method.contains(m)) {
            return Ok(req);
        }

        // 从 metadata 提取 Bearer token
        let token = extract_bearer_token(&req)?;

        // 通过 sa-token 验证 token
        let login_id = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                StpUtil::get_login_id(&token).await
            })
        })
        .map_err(|e| {
            warn!("gRPC 认证失败: {}", e);
            Status::unauthenticated(format!("认证失败: {}", e))
        })?;

        // 将 login_id 和 token 注入 request extensions
        req.extensions_mut().insert(GrpcLoginId(login_id));
        req.extensions_mut().insert(GrpcToken(token));

        Ok(req)
    }
}

/// 从 gRPC metadata 提取 Bearer token
fn extract_bearer_token(req: &Request<()>) -> Result<TokenValue, Status> {
    let auth_value = req
        .metadata()
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("缺少 authorization metadata"))?;

    let auth_str = auth_value
        .to_str()
        .map_err(|_| Status::unauthenticated("authorization metadata 格式无效"))?;

    if let Some(token) = auth_str.strip_prefix("Bearer ") {
        Ok(TokenValue::new(token))
    } else {
        Err(Status::unauthenticated(
            "authorization 格式应为 'Bearer {token}'",
        ))
    }
}

/// 从 request extensions 获取 login_id
///
/// 在 gRPC service 方法中调用，获取当前已认证用户的 login_id。
pub fn get_login_id(req: &Request<impl std::any::Any>) -> Result<String, Status> {
    req.extensions()
        .get::<GrpcLoginId>()
        .map(|id| id.0.clone())
        .ok_or_else(|| Status::unauthenticated("未找到登录信息"))
}

/// 从 request extensions 获取 login_id 为 u64
pub fn get_login_id_u64(req: &Request<impl std::any::Any>) -> Result<u64, Status> {
    let id_str = get_login_id(req)?;
    id_str
        .parse::<u64>()
        .map_err(|_| Status::internal("login_id 格式无效"))
}

/// 从 request extensions 获取原始 token
pub fn get_token(req: &Request<impl std::any::Any>) -> Result<TokenValue, Status> {
    req.extensions()
        .get::<GrpcToken>()
        .map(|t| t.0.clone())
        .ok_or_else(|| Status::unauthenticated("未找到 token"))
}

/// gRPC 权限检查
///
/// 通过 sa-token 检查用户是否拥有指定权限。
/// admin 角色（role_code="admin"）跳过权限检查。
pub async fn ensure_grpc_permission(login_id: &str, perm: &str) -> Result<(), Status> {
    // admin 角色跳过权限检查
    if StpUtil::has_role(login_id, crate::auth::ADMIN_ROLE).await {
        return Ok(());
    }
    StpUtil::check_permission(login_id, perm)
        .await
        .map_err(|e| Status::permission_denied(format!("缺少权限 {}: {}", perm, e)))
}

/// gRPC 角色检查
pub async fn ensure_grpc_role(login_id: &str, role: &str) -> Result<(), Status> {
    StpUtil::check_role(login_id, role)
        .await
        .map_err(|e| Status::permission_denied(format!("缺少角色 {}: {}", role, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataValue;

    /// 测试提取 Bearer token - 正常情况
    ///
    /// 验证：从正确的 "Bearer {token}" 格式中提取 token
    #[test]
    fn test_extract_bearer_token_success() {
        let mut req = Request::new(());
        req.metadata_mut().insert(
            "authorization",
            MetadataValue::from_static("Bearer abc123xyz"),
        );

        let result = extract_bearer_token(&req);
        assert!(result.is_ok(), "应该成功提取 token");

        let token = result.unwrap();
        assert_eq!(token.to_string(), "abc123xyz", "token 值应为 abc123xyz");
    }

    /// 测试提取 Bearer token - 缺少 authorization header
    ///
    /// 验证：没有 authorization header 时返回错误
    #[test]
    fn test_extract_bearer_token_missing() {
        let req = Request::new(());

        let result = extract_bearer_token(&req);
        assert!(result.is_err(), "缺少 authorization 应返回错误");

        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            tonic::Code::Unauthenticated,
            "错误码应为 Unauthenticated"
        );
        assert!(
            status.message().contains("缺少 authorization"),
            "错误信息应提示缺少 authorization"
        );
    }

    /// 测试提取 Bearer token - 格式错误（没有 Bearer 前缀）
    ///
    /// 验证：authorization 值不是 "Bearer xxx" 格式时返回错误
    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut req = Request::new(());
        req.metadata_mut().insert(
            "authorization",
            MetadataValue::from_static("Basic abc123"),
        );

        let result = extract_bearer_token(&req);
        assert!(result.is_err(), "格式错误应返回错误");

        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            tonic::Code::Unauthenticated,
            "错误码应为 Unauthenticated"
        );
        assert!(
            status.message().contains("Bearer"),
            "错误信息应提示需要 Bearer 格式"
        );
    }
}
