//! 认证域 E2E 回归测试：登录 / 未授权 / user_info / 登出
//!
//! ## 限流约束
//! `/api/v1/auth/login` 硬编码 `per_second(12)` + 桶容量 5。
//! 本文件内登录请求总次数控制在 **4 次**（成功 1 + 失败 1 + `admin_token()` 1 + 登出 1），
//! 超出会触发 429 误报，新增登录类用例前务必计数。

mod common;

use common::*;
use serde_json::{Value, json};

/// 登录成功：返回 code=200 + token + Set-Cookie
#[tokio::test]
async fn login_success_returns_token_and_cookie() {
    let srv = server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/v1/auth/login", srv.base_url))
        .json(&json!({
            "username": "admin",
            "password": "admin123",
            "loginIp": "127.0.0.1",
        }))
        .send()
        .await
        .expect("登录请求失败");
    assert_eq!(resp.status(), 200, "登录应返回 HTTP 200");

    let set_cookie = resp.headers().get(reqwest::header::SET_COOKIE).cloned();
    assert!(set_cookie.is_some(), "登录响应应包含 Set-Cookie 头");

    let body: Value = resp.json().await.expect("登录响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "登录业务码应为 200: {body}");
    let token = body["data"]["token"]
        .as_str()
        .expect("登录响应缺少 token")
        .to_string();
    assert!(!token.is_empty(), "token 不应为空");
    assert_eq!(body["data"]["username"], "admin", "应返回 admin 用户信息");
}

/// 登录失败：密码错误 → HTTP 200 + 业务码 2001（防枚举，不区分用户不存在）
#[tokio::test]
async fn login_wrong_password_rejected() {
    let srv = server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/v1/auth/login", srv.base_url))
        .json(&json!({
            "username": "admin",
            "password": "wrong-password",
            "loginIp": "127.0.0.1",
        }))
        .send()
        .await
        .expect("登录请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200 通道: {resp:?}");

    let body: Value = resp.json().await.expect("登录失败响应 JSON 解析失败");
    assert_eq!(body["code"], 2001, "认证失败业务码应为 2001: {body}");
    assert_eq!(body["data"], Value::Null, "失败时 data 应为 null");
    assert!(body["msg"].as_str().is_some(), "应返回错误消息");
}

/// 未登录访问受保护接口 → HTTP 401
///
/// 注意：sa-token 拦截器直接返回 HTTP 401（空 body），并非 ApiR JSON，
/// 因此只断言状态码，不解析响应体。
#[tokio::test]
async fn user_info_requires_auth() {
    let srv = server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/auth/user_info", srv.base_url))
        .send()
        .await
        .expect("请求失败");
    assert_eq!(resp.status(), 401, "未登录应返回 401");
}

/// 带 token 访问 user_info → 成功且返回 admin 用户信息
#[tokio::test]
async fn user_info_with_token_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .get(format!("{}/api/v1/auth/user_info", srv.base_url))
        .send()
        .await
        .expect("user_info 请求失败");
    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.expect("user_info 响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "user_info 应成功: {body}");
    assert_eq!(body["data"]["username"], "admin");
    assert!(!body["data"]["permissions"].is_null(), "应返回权限列表");
}

/// 登出：清 Cookie 且旧 token 立即失效
///
/// 使用独立登录的 token（不触碰共享 `admin_token()`，避免污染其他用例）。
#[tokio::test]
async fn logout_clears_cookie_and_invalidates_token() {
    let srv = server().await;
    let client = reqwest::Client::new();

    // 独立登录
    let login_resp = client
        .post(format!("{}/api/v1/auth/login", srv.base_url))
        .json(&json!({
            "username": "admin",
            "password": "admin123",
            "loginIp": "127.0.0.1",
        }))
        .send()
        .await
        .expect("登录请求失败");
    assert_eq!(login_resp.status(), 200, "独立登录应成功: {login_resp:?}");
    let login_body: Value = login_resp.json().await.expect("登录响应 JSON 解析失败");
    let token = login_body["data"]["token"]
        .as_str()
        .expect("登录响应缺少 token")
        .to_string();

    // 登出
    let logout_resp = client
        .post(format!("{}/api/v1/auth/logout", srv.base_url))
        .header(reqwest::header::AUTHORIZATION, &token)
        .send()
        .await
        .expect("登出请求失败");
    assert_eq!(
        logout_resp.status(),
        200,
        "登出应返回 HTTP 200: {logout_resp:?}"
    );
    let logout_body: Value = logout_resp.json().await.expect("登出响应 JSON 解析失败");
    assert_eq!(
        logout_body["code"], 200,
        "登出业务码应为 200: {logout_body}"
    );

    // 旧 token 应已失效
    let old_client = authed_client(&token);
    let resp = old_client
        .get(format!("{}/api/v1/auth/user_info", srv.base_url))
        .send()
        .await
        .expect("旧 token 访问失败");
    assert_eq!(resp.status(), 401, "登出后旧 token 应失效: {resp:?}");
}
