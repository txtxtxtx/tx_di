//! 用户域 E2E 回归测试：用户 CRUD / 分页 / 状态变更 / 改密 / 角色与部门分配
//!
//! ## 设计约束
//! - 登录限流桶容量 5：本文件全部用例复用共享 `admin_token()`（仅 1 次登录）。
//! - admin 角色（seed 已绑定）经 `ensure_permission` 直通全部 `user:*` 权限码。
//! - 每个用例创建独立用户（唯一 username），互不干扰、可并行。
//! - `u64` 字段（id/userId/tenantId 等）序列化为 JSON 字符串。

mod common;

use common::*;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成唯一用户名（时间戳纳秒），避免并行用例冲突
fn unique_username(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时钟应晚于 1970")
        .as_nanos();
    format!("e2e_{tag}_{nanos}")
}

/// 生成唯一 11 位手机号（基于时间戳，避免唯一约束冲突）
fn unique_mobile() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时钟应晚于 1970")
        .as_nanos();
    let tail = nanos % 10_000_000_000;
    format!("1{tail:010}")
}

/// 创建用户并返回 `{id, username}`，失败时 panic（供各用例复用）
async fn create_user(client: &reqwest::Client, base: &str, username: &str) -> (String, String) {
    let resp = client
        .post(format!("{base}/api/v1/user"))
        .json(&json!({
            "username": username,
            "password": "Test@123456",
            "nickname": "E2E 测试用户",
            "email": format!("{username}@example.com"),
            "mobile": unique_mobile(),
            "sex": 1,
            "remark": "created by e2e",
            "roleIds": [],
            "deptIds": [],
        }))
        .send()
        .await
        .expect("创建用户请求失败");
    assert_eq!(resp.status(), 200, "创建用户应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建用户业务码应为 200: {body}");
    let id = body["data"]["id"]
        .as_str()
        .map(str::to_string)
        .or_else(|| body["data"]["id"].as_u64().map(|v| v.to_string()))
        .expect("缺少用户 id");
    (id, username.to_string())
}

/// 创建用户成功：返回 id/username/默认状态
#[tokio::test]
async fn create_user_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("create");
    let (id, uname) = create_user(&client, &srv.base_url, &username).await;

    assert!(!id.is_empty(), "用户 id 不应为空");
    assert_eq!(uname, username);

    // 回查确认状态默认值（status=0 正常、sex=1）
    let resp = client
        .get(format!("{}/api/v1/user/{id}", srv.base_url))
        .send()
        .await
        .expect("回查用户请求失败");
    let body: Value = resp.json().await.expect("回查用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "回查用户应成功: {body}");
    assert_eq!(body["data"]["status"], 0, "新建用户状态应为 0(正常)");
    assert_eq!(body["data"]["sex"], 1, "新建用户性别应为 1(男)");
}

/// 创建用户：用户名重复 → 业务码 10201（用户名已存在）
#[tokio::test]
async fn create_user_duplicate_username_rejected() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("dup");
    let _ = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .post(format!("{}/api/v1/user", srv.base_url))
        .json(&json!({
            "username": username,
            "password": "Test@123456",
            "nickname": "重复用户",
            "roleIds": [],
            "deptIds": [],
        }))
        .send()
        .await
        .expect("重复创建请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("重复创建响应 JSON 解析失败");
    assert_eq!(body["code"], 10201, "重复用户名业务码应为 10201: {body}");
}

/// 按 id 查询：存在返回用户详情
#[tokio::test]
async fn get_user_by_id_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("get");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .get(format!("{}/api/v1/user/{id}", srv.base_url))
        .send()
        .await
        .expect("查询用户请求失败");
    let body: Value = resp.json().await.expect("查询用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "查询用户应成功: {body}");
    assert_eq!(body["data"]["username"], username);
    assert_eq!(body["data"]["id"].as_str(), Some(id.as_str()));
}

/// 按 id 查询：不存在 → 业务码 10101（记录不存在）
#[tokio::test]
async fn get_user_not_found() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .get(format!("{}/api/v1/user/999999999", srv.base_url))
        .send()
        .await
        .expect("查询不存在用户请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("查询不存在用户响应 JSON 解析失败");
    assert_eq!(body["code"], 10101, "不存在的用户业务码应为 10101: {body}");
}

/// 分页查询：返回 list/page/size/total，且新建用户可在列表中
#[tokio::test]
async fn list_users_paged_contains_new_user() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("page");
    let _ = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .post(format!("{}/api/v1/user/list", srv.base_url))
        .json(&json!({
            "username": username,
            "pageInfo": { "page": 1, "size": 10 },
        }))
        .send()
        .await
        .expect("分页查询请求失败");
    assert_eq!(resp.status(), 200, "分页查询应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("分页查询响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分页查询业务码应为 200: {body}");
    assert_eq!(body["data"]["total"], 1, "按 username 精确过滤应命中 1 条: {body}");
    let list = body["data"]["list"].as_array().expect("list 应为数组");
    assert_eq!(list.len(), 1, "list 应包含 1 条: {body}");
    assert_eq!(list[0]["username"], username);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["size"], 10);
}

/// 更新用户：修改 nickname/email 生效
#[tokio::test]
async fn update_user_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("upd");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .put(format!("{}/api/v1/user/{id}", srv.base_url))
        .json(&json!({
            "userId": id,
            "nickname": "E2E 更新后的昵称",
            "remark": "updated by e2e",
        }))
        .send()
        .await
        .expect("更新用户请求失败");
    assert_eq!(resp.status(), 200, "更新用户应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("更新用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "更新用户业务码应为 200: {body}");
    assert_eq!(body["data"]["nickname"], "E2E 更新后的昵称");
    assert_eq!(body["data"]["remark"], "updated by e2e");
}

/// 状态变更：change_status 禁用（status=1）后生效
#[tokio::test]
async fn change_status_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("status");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .post(format!("{}/api/v1/user/change_status", srv.base_url))
        .json(&json!({ "userId": id, "status": 1 }))
        .send()
        .await
        .expect("变更状态请求失败");
    assert_eq!(resp.status(), 200, "变更状态应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("变更状态响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "变更状态业务码应为 200: {body}");
    assert_eq!(body["data"]["status"], 1, "变更后状态应为 1(禁用): {body}");

    // 回查确认持久化
    let resp = client
        .get(format!("{}/api/v1/user/{id}", srv.base_url))
        .send()
        .await
        .expect("回查用户请求失败");
    let body: Value = resp.json().await.expect("回查用户响应 JSON 解析失败");
    assert_eq!(body["data"]["status"], 1, "禁用状态应持久化: {body}");
}

/// 查询用户状态码（0-正常, 1-禁用, 2-锁定），失败返回 -1
async fn get_user_status(
    client: &reqwest::Client,
    base: &str,
    user_id: &str,
) -> i64 {
    let resp = client
        .get(format!("{base}/api/v1/user/{user_id}"))
        .send()
        .await
        .expect("回查用户请求失败");
    let body: Value = resp.json().await.expect("回查用户响应 JSON 解析失败");
    body["data"]["status"].as_i64().unwrap_or(-1)
}

/// 状态快捷接口：disable/lock/unlock 依次生效
#[tokio::test]
async fn status_shortcuts_disable_lock_unlock() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("scut");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    // disable → 1
    let resp = client
        .post(format!("{}/api/v1/user/disable", srv.base_url))
        .json(&json!({ "userId": id }))
        .send()
        .await
        .expect("disable 请求失败");
    assert_eq!(resp.status(), 200, "disable 应返回 HTTP 200: {resp:?}");
    assert_eq!(get_user_status(&client, &srv.base_url, &id).await, 1, "disable 后状态应为 1");

    // lock → 2
    let resp = client
        .post(format!("{}/api/v1/user/lock", srv.base_url))
        .json(&json!({ "userId": id }))
        .send()
        .await
        .expect("lock 请求失败");
    assert_eq!(resp.status(), 200, "lock 应返回 HTTP 200: {resp:?}");
    assert_eq!(get_user_status(&client, &srv.base_url, &id).await, 2, "lock 后状态应为 2");

    // unlock → 0（unlock 置为 Active）
    let resp = client
        .post(format!("{}/api/v1/user/unlock", srv.base_url))
        .json(&json!({ "userId": id }))
        .send()
        .await
        .expect("unlock 请求失败");
    assert_eq!(resp.status(), 200, "unlock 应返回 HTTP 200: {resp:?}");
    assert_eq!(get_user_status(&client, &srv.base_url, &id).await, 0, "unlock 后状态应为 0");
}

/// 修改密码：成功返回 200
#[tokio::test]
async fn change_password_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("pwd");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .post(format!("{}/api/v1/user/change_password", srv.base_url))
        .json(&json!({ "userId": id, "newPassword": "NewPass@123456" }))
        .send()
        .await
        .expect("修改密码请求失败");
    assert_eq!(resp.status(), 200, "修改密码应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("修改密码响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "修改密码业务码应为 200: {body}");
}

/// 分配角色与部门：assign_roles / assign_depts 成功
#[tokio::test]
async fn assign_roles_and_depts_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("assign");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    // 分配角色（seed 角色 id=1：admin）
    let resp = client
        .post(format!("{}/api/v1/user/assign_roles", srv.base_url))
        .json(&json!({ "userId": id, "roleIds": ["1"] }))
        .send()
        .await
        .expect("分配角色请求失败");
    assert_eq!(resp.status(), 200, "分配角色应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("分配角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分配角色业务码应为 200: {body}");

    // 分配部门（seed 部门 id=1：总公司）
    let resp = client
        .post(format!("{}/api/v1/user/assign_depts", srv.base_url))
        .json(&json!({ "userId": id, "deptIds": ["1"] }))
        .send()
        .await
        .expect("分配部门请求失败");
    assert_eq!(resp.status(), 200, "分配部门应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("分配部门响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分配部门业务码应为 200: {body}");

    // 回查确认绑定
    let resp = client
        .get(format!("{}/api/v1/user/{id}", srv.base_url))
        .send()
        .await
        .expect("回查用户请求失败");
    let body: Value = resp.json().await.expect("回查用户响应 JSON 解析失败");
    let role_ids = body["data"]["roleIds"]
        .as_array()
        .expect("roleIds 应为数组");
    assert!(
        role_ids.iter().any(|v| v.as_str() == Some("1")),
        "应绑定角色 1: {body}"
    );
}

/// 删除用户：删除后查询返回 10101（记录不存在）
#[tokio::test]
async fn delete_user_then_not_found() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let username = unique_username("del");
    let (id, _) = create_user(&client, &srv.base_url, &username).await;

    let resp = client
        .delete(format!("{}/api/v1/user/{id}", srv.base_url))
        .send()
        .await
        .expect("删除用户请求失败");
    assert_eq!(resp.status(), 200, "删除用户应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("删除用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "删除用户业务码应为 200: {body}");

    // 删除后查询 → 记录不存在
    let resp = client
        .get(format!("{}/api/v1/user/{id}", srv.base_url))
        .send()
        .await
        .expect("查询已删除用户请求失败");
    let body: Value = resp.json().await.expect("查询已删除用户响应 JSON 解析失败");
    assert_eq!(body["code"], 10101, "删除后查询应返回 10101: {body}");
}

/// 未登录访问用户域接口 → HTTP 401
#[tokio::test]
async fn user_api_requires_auth() {
    let srv = server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/user/1", srv.base_url))
        .send()
        .await
        .expect("请求失败");
    assert_eq!(resp.status(), 401, "未登录应返回 401: {resp:?}");
}
