//! RBAC 域 E2E 回归测试：角色 / 菜单 / 部门
//!
//! ## 设计约束
//! - 登录限流桶容量 5：本文件全部用例复用共享 `admin_token()`（仅 1 次登录）。
//! - admin 角色（seed id=1）绑定全部菜单，`ensure_permission` 直通 role:/menu:/dept: 权限码。
//! - 每个用例创建独立实体（唯一 code/name），互不干扰、可并行。
//! - 业务错误走 HTTP 200 + `code` 业务码；认证失败 HTTP 401。
//! - 分页请求用顶层 `page`+`pageSize`；分页响应为 `Page{list,page,size,total}`。
//! - proto DTO 的 u64 id 序列化为字符串；树节点 id 视结构而定（用 extract_id 兼容）。

mod common;

use common::*;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成唯一标识（时间戳纳秒），避免并行用例冲突
fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时钟应晚于 1970")
        .as_nanos()
        .to_string()
}

/// 健壮提取 id（proto DTO 为字符串，树节点可能为数字）
fn extract_id(v: &Value) -> Option<String> {
    v.as_str()
        .map(str::to_string)
        .or_else(|| v.as_u64().map(|x| x.to_string()))
}

/// 创建角色，返回 id
async fn create_role(client: &reqwest::Client, base: &str, code: &str) -> String {
    let resp = client
        .post(format!("{base}/api/v1/role"))
        .json(&json!({
            "name": format!("角色 {code}"),
            "code": code,
            "sort": 10,
            "remark": "created by e2e",
            "menuIds": [],
        }))
        .send()
        .await
        .expect("创建角色请求失败");
    assert_eq!(resp.status(), 200, "创建角色应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建角色业务码应为 200: {body}");
    extract_id(&body["data"]["id"]).expect("创建角色缺少 id")
}

/// 创建菜单（types=1 菜单），返回 id
async fn create_menu(client: &reqwest::Client, base: &str, name: &str, parent_id: u64) -> String {
    let resp = client
        .post(format!("{base}/api/v1/menu"))
        .json(&json!({
            "name": name,
            "permission": format!("e2e:{}", unique_suffix()),
            "types": 1,
            "sort": 10,
            "parentId": parent_id,
            "path": "/e2e-menu",
            "icon": "",
            "component": "",
            "componentName": "",
        }))
        .send()
        .await
        .expect("创建菜单请求失败");
    assert_eq!(resp.status(), 200, "创建菜单应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建菜单响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建菜单业务码应为 200: {body}");
    extract_id(&body["data"]["id"]).expect("创建菜单缺少 id")
}

/// 创建部门（parent_id=0 顶级），返回 id
async fn create_dept(client: &reqwest::Client, base: &str, name: &str) -> String {
    let resp = client
        .post(format!("{base}/api/v1/dept"))
        .json(&json!({
            "name": name,
            "parentId": 0,
            "sort": 10,
            "leaderUserId": 1,
            "phone": "",
            "email": "",
        }))
        .send()
        .await
        .expect("创建部门请求失败");
    assert_eq!(resp.status(), 200, "创建部门应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建部门响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建部门业务码应为 200: {body}");
    extract_id(&body["data"]["id"]).expect("创建部门缺少 id")
}

// ============================================================================
// 角色域
// ============================================================================

#[tokio::test]
async fn role_create_and_get_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_role_{}", unique_suffix());
    let id = create_role(&client, &srv.base_url, &code).await;
    assert!(!id.is_empty());

    let resp = client
        .get(format!("{}/api/v1/role/{id}", srv.base_url))
        .send()
        .await
        .expect("回查角色请求失败");
    let body: Value = resp.json().await.expect("回查角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "回查角色应成功: {body}");
    assert_eq!(body["data"]["code"], code, "角色 code 应匹配: {body}");
    assert_eq!(body["data"]["name"], format!("角色 {code}"));
}

#[tokio::test]
async fn role_duplicate_code_rejected() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_dup_{}", unique_suffix());
    let _ = create_role(&client, &srv.base_url, &code).await;

    let resp = client
        .post(format!("{}/api/v1/role", srv.base_url))
        .json(&json!({
            "name": "重复角色",
            "code": code,
            "sort": 11,
            "menuIds": [],
        }))
        .send()
        .await
        .expect("重复创建角色请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("重复创建角色响应 JSON 解析失败");
    assert_eq!(
        body["code"], 10202,
        "重复角色 code 业务码应为 10202: {body}"
    );
}

#[tokio::test]
async fn role_get_not_found() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .get(format!("{}/api/v1/role/999999999", srv.base_url))
        .send()
        .await
        .expect("查询不存在角色请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("查询不存在角色响应 JSON 解析失败");
    assert_eq!(body["code"], 10102, "不存在的角色业务码应为 10102: {body}");
}

#[tokio::test]
async fn role_list_paged_contains_new() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_page_{}", unique_suffix());
    let _ = create_role(&client, &srv.base_url, &code).await;

    let resp = client
        .post(format!("{}/api/v1/role/list", srv.base_url))
        .json(&json!({ "page": 1, "pageSize": 10, "name": "", "code": "", "status": 0 }))
        .send()
        .await
        .expect("分页查询角色请求失败");
    let body: Value = resp.json().await.expect("分页查询角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分页查询角色应成功: {body}");
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["size"], 10);
    let list = body["data"]["list"].as_array().expect("list 应为数组");
    assert!(
        list.iter().any(|r| r["code"] == code),
        "列表中应包含新创建角色: {body}"
    );
}

#[tokio::test]
async fn role_update_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_upd_{}", unique_suffix());
    let id = create_role(&client, &srv.base_url, &code).await;

    let resp = client
        .put(format!("{}/api/v1/role/{id}", srv.base_url))
        .json(&json!({
            "roleId": id,
            "name": "更新后的角色",
            "code": code,
            "sort": 20,
            "dataScope": 1,
            "remark": "updated by e2e",
        }))
        .send()
        .await
        .expect("更新角色请求失败");
    let body: Value = resp.json().await.expect("更新角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "更新角色应成功: {body}");
    assert_eq!(body["data"]["name"], "更新后的角色");
    assert_eq!(body["data"]["sort"], 20);
}

#[tokio::test]
async fn role_assign_menus_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_am_{}", unique_suffix());
    let id = create_role(&client, &srv.base_url, &code).await;

    // 分配 seed 菜单 id=11（系统管理目录）
    let resp = client
        .post(format!("{}/api/v1/role/assign_menus", srv.base_url))
        .json(&json!({ "roleId": id, "menuIds": ["11"] }))
        .send()
        .await
        .expect("分配菜单请求失败");
    assert_eq!(resp.status(), 200, "分配菜单应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("分配菜单响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分配菜单业务码应为 200: {body}");

    // 回查确认 menuIds 已绑定
    let resp = client
        .get(format!("{}/api/v1/role/{id}", srv.base_url))
        .send()
        .await
        .expect("回查角色请求失败");
    let body: Value = resp.json().await.expect("回查角色响应 JSON 解析失败");
    let menu_ids = body["data"]["menuIds"]
        .as_array()
        .expect("menuIds 应为数组");
    assert!(
        menu_ids.iter().any(|m| m.as_str() == Some("11")),
        "角色应绑定菜单 11: {body}"
    );
}

#[tokio::test]
async fn role_get_all_contains_admin_seed() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .get(format!("{}/api/v1/role/all", srv.base_url))
        .send()
        .await
        .expect("获取全部角色请求失败");
    let body: Value = resp.json().await.expect("获取全部角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "获取全部角色应成功: {body}");
    let list = body["data"].as_array().expect("data 应为数组");
    assert!(
        list.iter().any(|r| r["code"] == "admin"),
        "全部角色中应包含 seed admin 角色: {body}"
    );
}

#[tokio::test]
async fn role_add_and_remove_users_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_ru_{}", unique_suffix());
    let id = create_role(&client, &srv.base_url, &code).await;

    // 添加用户（裸数字数组，body 为 Json<Vec<u64>>）；admin 用户 id=1
    let resp = client
        .post(format!("{}/api/v1/role/{id}/users", srv.base_url))
        .json(&json!([1]))
        .send()
        .await
        .expect("添加角色用户请求失败");
    assert_eq!(resp.status(), 200, "添加角色用户应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("添加角色用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "添加角色用户业务码应为 200: {body}");

    // 回查角色下用户
    let resp = client
        .get(format!("{}/api/v1/role/{id}/users", srv.base_url))
        .send()
        .await
        .expect("回查角色用户请求失败");
    let body: Value = resp.json().await.expect("回查角色用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "回查角色用户应成功: {body}");
    let users = body["data"].as_array().expect("data 应为数组");
    assert!(
        users.iter().any(|u| u["id"].as_str() == Some("1")),
        "角色下应包含 admin 用户: {body}"
    );

    // 移除用户
    let resp = client
        .delete(format!("{}/api/v1/role/{id}/users", srv.base_url))
        .json(&json!([1]))
        .send()
        .await
        .expect("移除角色用户请求失败");
    let body: Value = resp.json().await.expect("移除角色用户响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "移除角色用户应成功: {body}");
}

#[tokio::test]
async fn role_delete_then_not_found() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let code = format!("e2e_del_{}", unique_suffix());
    let id = create_role(&client, &srv.base_url, &code).await;

    let resp = client
        .delete(format!("{}/api/v1/role/{id}", srv.base_url))
        .send()
        .await
        .expect("删除角色请求失败");
    assert_eq!(resp.status(), 200, "删除角色应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("删除角色响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "删除角色业务码应为 200: {body}");

    let resp = client
        .get(format!("{}/api/v1/role/{id}", srv.base_url))
        .send()
        .await
        .expect("查询已删除角色请求失败");
    let body: Value = resp.json().await.expect("查询已删除角色响应 JSON 解析失败");
    assert_eq!(body["code"], 10102, "删除后查询应返回 10102: {body}");
}

// ============================================================================
// 菜单域
// ============================================================================

#[tokio::test]
async fn menu_create_and_get_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E菜单{}", unique_suffix());
    let id = create_menu(&client, &srv.base_url, &name, 11).await;
    assert!(!id.is_empty());

    let resp = client
        .get(format!("{}/api/v1/menu/{id}", srv.base_url))
        .send()
        .await
        .expect("回查菜单请求失败");
    let body: Value = resp.json().await.expect("回查菜单响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "回查菜单应成功: {body}");
    assert_eq!(body["data"]["name"], name);
    assert_eq!(body["data"]["types"], 1);
}

#[tokio::test]
async fn menu_list_tree_contains_seed() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .post(format!("{}/api/v1/menu/list", srv.base_url))
        .json(&json!({ "name": "", "status": 0 }))
        .send()
        .await
        .expect("查询菜单树请求失败");
    let body: Value = resp.json().await.expect("查询菜单树响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "查询菜单树应成功: {body}");
    let tree = body["data"].as_array().expect("data 应为树数组");
    assert!(
        tree.iter().any(|m| m["name"] == "系统管理"),
        "菜单树应包含 seed 系统管理: {body}"
    );
}

#[tokio::test]
async fn menu_update_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E更新{}", unique_suffix());
    let id = create_menu(&client, &srv.base_url, name.as_str(), 11).await;

    let resp = client
        .put(format!("{}/api/v1/menu/{id}", srv.base_url))
        .json(&json!({
            "menuId": id,
            "name": name,
            "permission": format!("e2e:{}", unique_suffix()),
            "types": 1,
            "sort": 30,
            "parentId": 11,
            "path": "/e2e-updated",
            "icon": "",
            "component": "",
            "componentName": "",
            "visible": 0,
            "keepAlive": 0,
        }))
        .send()
        .await
        .expect("更新菜单请求失败");
    let body: Value = resp.json().await.expect("更新菜单响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "更新菜单应成功: {body}");
    assert_eq!(body["data"]["sort"], 30);
}

#[tokio::test]
async fn menu_delete_has_children_rejected() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E父菜单{}", unique_suffix());
    let parent_id = create_menu(&client, &srv.base_url, name.as_str(), 11).await;
    let parent_num: u64 = parent_id.parse().expect("父菜单 id 应为数字");
    // 建子菜单
    let child_name = format!("E2E子菜单{}", unique_suffix());
    let _ = create_menu(&client, &srv.base_url, child_name.as_str(), parent_num).await;

    let resp = client
        .delete(format!("{}/api/v1/menu/{parent_id}", srv.base_url))
        .send()
        .await
        .expect("删除含子菜单的菜单请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp
        .json()
        .await
        .expect("删除含子菜单的菜单响应 JSON 解析失败");
    assert_eq!(body["code"], 10307, "含子菜单删除应返回 10307: {body}");
}

#[tokio::test]
async fn menu_delete_leaf_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E叶子{}", unique_suffix());
    let id = create_menu(&client, &srv.base_url, name.as_str(), 11).await;

    let resp = client
        .delete(format!("{}/api/v1/menu/{id}", srv.base_url))
        .send()
        .await
        .expect("删除叶子菜单请求失败");
    assert_eq!(resp.status(), 200, "删除叶子菜单应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("删除叶子菜单响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "删除叶子菜单业务码应为 200: {body}");
}

// ============================================================================
// 部门域
// ============================================================================

#[tokio::test]
async fn dept_create_and_get_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E部门{}", unique_suffix());
    let id = create_dept(&client, &srv.base_url, &name).await;
    assert!(!id.is_empty());

    let resp = client
        .get(format!("{}/api/v1/dept/{id}", srv.base_url))
        .send()
        .await
        .expect("回查部门请求失败");
    let body: Value = resp.json().await.expect("回查部门响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "回查部门应成功: {body}");
    assert_eq!(body["data"]["name"], name);
}

#[tokio::test]
async fn dept_get_not_found() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .get(format!("{}/api/v1/dept/999999999", srv.base_url))
        .send()
        .await
        .expect("查询不存在部门请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("查询不存在部门响应 JSON 解析失败");
    assert_eq!(body["code"], 10103, "不存在的部门业务码应为 10103: {body}");
}

#[tokio::test]
async fn dept_list_tree_contains_seed() {
    let srv = server().await;
    let client = authed_client(admin_token().await);

    let resp = client
        .post(format!("{}/api/v1/dept/list", srv.base_url))
        .json(&json!({ "name": "", "status": 0 }))
        .send()
        .await
        .expect("查询部门树请求失败");
    let body: Value = resp.json().await.expect("查询部门树响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "查询部门树应成功: {body}");
    let tree = body["data"].as_array().expect("data 应为树数组");
    assert!(
        tree.iter().any(|d| d["name"] == "总公司"),
        "部门树应包含 seed 总公司: {body}"
    );
}

#[tokio::test]
async fn dept_update_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E更新部门{}", unique_suffix());
    let id = create_dept(&client, &srv.base_url, &name).await;

    let resp = client
        .put(format!("{}/api/v1/dept/{id}", srv.base_url))
        .json(&json!({
            "deptId": id,
            "name": name,
            "parentId": 0,
            "sort": 40,
            "leaderUserId": 1,
            "phone": "",
            "email": "",
        }))
        .send()
        .await
        .expect("更新部门请求失败");
    let body: Value = resp.json().await.expect("更新部门响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "更新部门应成功: {body}");
    assert_eq!(body["data"]["sort"], 40);
}

#[tokio::test]
async fn dept_delete_has_children_rejected() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E父部门{}", unique_suffix());
    let parent_id = create_dept(&client, &srv.base_url, &name).await;
    let parent_num: u64 = parent_id.parse().expect("父部门 id 应为数字");
    // 建子部门
    let child_name = format!("E2E子部门{}", unique_suffix());
    let resp = client
        .post(format!("{}/api/v1/dept", srv.base_url))
        .json(&json!({
            "name": child_name,
            "parentId": parent_num,
            "sort": 11,
            "leaderUserId": 1,
            "phone": "",
            "email": "",
        }))
        .send()
        .await
        .expect("创建子部门请求失败");
    let body: Value = resp.json().await.expect("创建子部门响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建子部门应成功: {body}");

    let resp = client
        .delete(format!("{}/api/v1/dept/{parent_id}", srv.base_url))
        .send()
        .await
        .expect("删除含子部门的部门请求失败");
    assert_eq!(resp.status(), 200, "业务错误走 HTTP 200: {resp:?}");
    let body: Value = resp
        .json()
        .await
        .expect("删除含子部门的部门响应 JSON 解析失败");
    assert_eq!(body["code"], 10309, "含子部门删除应返回 10309: {body}");
}

#[tokio::test]
async fn dept_delete_leaf_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let name = format!("E2E叶子部门{}", unique_suffix());
    let id = create_dept(&client, &srv.base_url, &name).await;

    let resp = client
        .delete(format!("{}/api/v1/dept/{id}", srv.base_url))
        .send()
        .await
        .expect("删除叶子部门请求失败");
    assert_eq!(resp.status(), 200, "删除叶子部门应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("删除叶子部门响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "删除叶子部门业务码应为 200: {body}");
}

// ============================================================================
// 认证约束
// ============================================================================

#[tokio::test]
async fn rbac_api_requires_auth() {
    let srv = server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/v1/role/1", srv.base_url))
        .send()
        .await
        .expect("请求失败");
    assert_eq!(resp.status(), 401, "未登录访问角色接口应 401: {resp:?}");

    let resp = client
        .get(format!("{}/api/v1/menu/11", srv.base_url))
        .send()
        .await
        .expect("请求失败");
    assert_eq!(resp.status(), 401, "未登录访问菜单接口应 401: {resp:?}");
}
