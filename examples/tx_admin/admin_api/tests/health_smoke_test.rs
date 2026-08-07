//! 批次 1：冒烟测试 —— App 启动链路 + health + 登录
//!
//! 覆盖：
//! - `/health`、`/health/live`、`/health/ready` 探活（纯 JSON，非 ApiR）
//! - 登录成功（seed 账号 admin/admin123）返回 token
//! - 未登录访问受保护接口返回 401
//! - 携带 token 访问受保护接口成功

mod common;

use common::{authed_client, login, server};
use serde_json::Value;

#[tokio::test]
async fn health_endpoints_ok() {
    let srv = server().await;

    // /health/live：存活探针
    let resp = reqwest::get(format!("{}/health/live", srv.base_url))
        .await
        .expect("health/live 请求失败");
    assert_eq!(resp.status(), 200, "health/live 状态码错误");
    let body: Value = resp.json().await.expect("health/live 响应 JSON 解析失败");
    assert_eq!(body["status"], "alive", "health/live 响应错误: {body}");

    // /health/ready：就绪探针（校验数据库）
    let resp = reqwest::get(format!("{}/health/ready", srv.base_url))
        .await
        .expect("health/ready 请求失败");
    assert_eq!(resp.status(), 200, "health/ready 状态码错误");
    let body: Value = resp.json().await.expect("health/ready 响应 JSON 解析失败");
    assert_eq!(body["db"], "ok", "health/ready 数据库未就绪: {body}");

    // /health：综合健康信息
    let resp = reqwest::get(format!("{}/health", srv.base_url))
        .await
        .expect("health 请求失败");
    assert_eq!(resp.status(), 200, "health 状态码错误");
    let body: Value = resp.json().await.expect("health 响应 JSON 解析失败");
    assert_eq!(body["status"], "ok", "health 状态错误: {body}");
}

#[tokio::test]
async fn login_success_returns_token() {
    let srv = server().await;
    let client = reqwest::Client::new();
    let token = login(&client, &srv.base_url).await.expect("登录失败");
    assert!(!token.is_empty(), "登录返回的 token 为空");
}

#[tokio::test]
async fn protected_api_requires_token() {
    let srv = server().await;
    let resp = reqwest::get(format!("{}/api/v1/auth/user_info", srv.base_url))
        .await
        .expect("请求失败");
    assert_eq!(resp.status(), 401, "未登录访问受保护接口应返回 401");
}

#[tokio::test]
async fn protected_api_with_token_ok() {
    let srv = server().await;
    let client = reqwest::Client::new();
    let token = login(&client, &srv.base_url).await.expect("登录失败");
    let authed = authed_client(&token);
    let resp = authed
        .get(format!("{}/api/v1/auth/user_info", srv.base_url))
        .send()
        .await
        .expect("请求失败");
    assert_eq!(resp.status(), 200, "携带 token 应返回 200");
    let body: Value = resp.json().await.expect("响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "业务码错误: {body}");
}
