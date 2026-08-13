//! gRPC AuthService 回归测试
//!
//! 复用 `common::server()` E2E 基座自动启动完整 App（含 gRPC 服务 + 随机端口），
//! 无需再手动 `cargo run` 起服务。覆盖登录成功/失败、获取用户信息、登出、鉴权拦截。
//!
//! ## 改造说明
//! 此前版本连接固定 `127.0.0.1:50051`，依赖外部手动启动服务，无法随 `cargo test`
//! 自动通过（属测试说明书缺口 G-2）。本次改为复用基座注入的随机 `grpc_url`，
//! 使其成为可自动运行的回归测试。

mod common;

use admin_proto::admin::auth::auth_service_client::AuthServiceClient;
use admin_proto::admin::auth::{GetUserInfoRequest, LoginRequest, LogoutRequest};
use admin_proto::admin::common::PageRequest;
use admin_proto::admin::user::user_service_client::UserServiceClient;
use admin_proto::admin::user::ListUsersRequest;
use tonic::Request;

/// 获取共享测试服务器（自动启动完整 App + gRPC 服务）
async fn server_grpc() -> &'static common::TestServer {
    common::server().await
}

/// 测试登录接口
///
/// 验证：
/// 1. 使用正确的用户名密码能成功登录
/// 2. 返回的 token 不为空
/// 3. 返回的 user_id 大于 0
#[tokio::test]
async fn test_login_success() {
    let srv = server_grpc().await;
    let mut client = AuthServiceClient::connect(srv.grpc_url.clone())
        .await
        .expect("无法连接到 gRPC 服务器");

    let request = Request::new(LoginRequest {
        username: "admin".to_string(),
        password: "admin123".to_string(),
        login_ip: "127.0.0.1".to_string(),
    });

    let response = client
        .login(request)
        .await
        .expect("登录请求失败")
        .into_inner();

    // 验证返回数据
    assert!(response.user_id > 0, "user_id 应该大于 0");
    assert!(!response.token.is_empty(), "token 不应为空");
    assert_eq!(response.username, "admin", "用户名应为 admin");
}

/// 测试登录失败 - 错误密码
///
/// 验证：使用错误密码登录时返回 Unauthenticated
#[tokio::test]
async fn test_login_wrong_password() {
    let srv = server_grpc().await;
    let mut client = AuthServiceClient::connect(srv.grpc_url.clone())
        .await
        .expect("无法连接到 gRPC 服务器");

    let request = Request::new(LoginRequest {
        username: "admin".to_string(),
        password: "wrong_password".to_string(),
        login_ip: "127.0.0.1".to_string(),
    });

    let result = client.login(request).await;

    assert!(result.is_err(), "错误密码应该返回错误");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::Unauthenticated,
        "错误码应为 Unauthenticated,{}",
        status.message()
    );
}

/// 测试获取用户信息接口
///
/// 验证：
/// 1. 先登录获取 token
/// 2. 使用 user_id 获取用户信息
/// 3. 返回的用户信息字段完整
#[tokio::test]
async fn test_get_user_info() {
    let srv = server_grpc().await;
    let mut client = AuthServiceClient::connect(srv.grpc_url.clone())
        .await
        .expect("无法连接到 gRPC 服务器");

    // 先登录获取 user_id
    let login_request = Request::new(LoginRequest {
        username: "admin".to_string(),
        password: "admin123".to_string(),
        login_ip: "127.0.0.1".to_string(),
    });

    let login_response = client
        .login(login_request)
        .await
        .expect("登录失败")
        .into_inner();

    // 获取用户信息（需要携带 token）
    let mut user_info_request = Request::new(GetUserInfoRequest {
        user_id: login_response.user_id,
    });
    user_info_request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", login_response.token).parse().unwrap(),
    );

    let user_info = client
        .get_user_info(user_info_request)
        .await
        .expect("获取用户信息失败")
        .into_inner();

    // 验证用户信息
    assert_eq!(user_info.user_id, login_response.user_id, "user_id 应一致");
    assert!(!user_info.username.is_empty(), "用户名不应为空");
    assert!(!user_info.nickname.is_empty(), "昵称不应为空");
}

/// 测试未携带 token 访问受保护接口被拦截
///
/// 验证：gRPC 鉴权拦截器（AuthLayer）对无 token 请求返回 Unauthenticated
#[tokio::test]
async fn test_get_user_info_without_token_rejected() {
    let srv = server_grpc().await;
    let mut client = UserServiceClient::connect(srv.grpc_url.clone())
        .await
        .expect("无法连接到 gRPC 服务器");

    let request = Request::new(ListUsersRequest {
        page_info: Some(PageRequest { page: 1, size: 10 }),
        ..Default::default()
    });

    let result = client.list_users(request).await;
    assert!(result.is_err(), "未携带 token 应被拦截");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::Unauthenticated,
        "无 token 应返回 Unauthenticated,{}",
        status.message()
    );
}

/// 测试登出接口
///
/// 验证：
/// 1. 先登录
/// 2. 调用登出接口
/// 3. 返回成功
#[tokio::test]
async fn test_logout() {
    let srv = server_grpc().await;
    let mut client = AuthServiceClient::connect(srv.grpc_url.clone())
        .await
        .expect("无法连接到 gRPC 服务器");

    // 先登录
    let login_request = Request::new(LoginRequest {
        username: "admin".to_string(),
        password: "admin123".to_string(),
        login_ip: "127.0.0.1".to_string(),
    });

    let login_response = client
        .login(login_request)
        .await
        .expect("登录失败")
        .into_inner();

    // 登出
    let mut logout_request = Request::new(LogoutRequest {
        user_id: login_response.user_id,
    });
    logout_request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", login_response.token).parse().unwrap(),
    );
    let response = client.logout(logout_request).await.expect("登出失败");

    // 验证返回成功
    assert_eq!(response.into_inner(), admin_proto::Empty {});
}
