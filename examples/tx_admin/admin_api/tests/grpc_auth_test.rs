//! gRPC AuthService 集成测试
//!
//! 测试登录和获取用户信息接口

use admin_proto::admin::auth::auth_service_client::AuthServiceClient;
use admin_proto::admin::auth::{GetUserInfoRequest, LoginRequest, LogoutRequest};
use tonic::Request;

/// 测试服务器地址（需要先启动服务）
const SERVER_URL: &str = "http://127.0.0.1:50051";

/// 测试登录接口
///
/// 验证：
/// 1. 使用正确的用户名密码能成功登录
/// 2. 返回的 token 不为空
/// 3. 返回的 user_id 大于 0
#[tokio::test]
async fn test_login_success() {
    let mut client = AuthServiceClient::connect(SERVER_URL)
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

    println!("✅ 登录成功: user_id={}, token={}", response.user_id, response.token);
}

/// 测试登录失败 - 错误密码
///
/// 验证：使用错误密码登录时返回错误
#[tokio::test]
async fn test_login_wrong_password() {
    let mut client = AuthServiceClient::connect(SERVER_URL)
        .await
        .expect("无法连接到 gRPC 服务器");

    let request = Request::new(LoginRequest {
        username: "admin".to_string(),
        password: "wrong_password".to_string(),
        login_ip: "127.0.0.1".to_string(),
    });

    let result = client.login(request).await;

    // 验证登录失败
    assert!(result.is_err(), "错误密码应该返回错误");

    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::Unauthenticated,
        "错误码应为 Unauthenticated,{}", status.message()
    );

    println!("✅ 登录失败测试通过: {}", status.message());
}

/// 测试获取用户信息接口
///
/// 验证：
/// 1. 先登录获取 token
/// 2. 使用 user_id 获取用户信息
/// 3. 返回的用户信息字段完整
#[tokio::test]
async fn test_get_user_info() {
    let mut client = AuthServiceClient::connect(SERVER_URL)
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

    println!(
        "✅ 获取用户信息成功: username={}, nickname={}, roles={:?}",
        user_info.username, user_info.nickname, user_info.roles
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
    let mut client = AuthServiceClient::connect(SERVER_URL)
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
    let response = client
        .logout(logout_request)
        .await
        .expect("登出失败");

    // 验证返回成功
    assert_eq!(response.into_inner(), admin_proto::Empty {});

    println!("✅ 登出成功");
}
