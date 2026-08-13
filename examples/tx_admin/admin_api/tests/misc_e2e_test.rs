//! Config / Dict / Log 域 HTTP E2E 回归测试（G-3 补齐）
//!
//! 覆盖此前缺失的 Config/Dict/Log 协议链路（路由 + 鉴权 + JSON 序列化 + 落库回查）。
//! 各模块业务逻辑已在 `admin_app` 集成测试覆盖，本文件补充 HTTP 层回归验证。
//!
//! ## 设计约束
//! - 复用共享 `admin_token()`（进程内仅 1 次登录，规避限流桶容量 5）。
//! - 每个用例创建唯一实体（时间戳后缀），互不干扰、可并行。
//! - 断言业务码 + 回查持久化，验证真实行为而非仅状态码。

mod common;

use common::*;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成唯一后缀（时间戳纳秒），避免并行用例冲突
fn unique_suffix(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时钟应晚于 1970")
        .as_nanos();
    format!("e2e_{tag}_{nanos}")
}

// ═══════════════════════════════ Config ═══════════════════════════════════

/// 创建配置成功：返回字段正确 + 回查按 key 命中
#[tokio::test]
async fn config_create_and_get_by_key_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let key = unique_suffix("cfgkey");
    let name = format!("系统名称-{key}");

    let resp = client
        .post(format!("{}/api/v1/config", srv.base_url))
        .json(&json!({
            "category": "system",
            "configType": 0,
            "name": name,
            "configKey": key,
            "value": "AdminSystem",
            "remark": "e2e",
        }))
        .send()
        .await
        .expect("创建配置请求失败");
    assert_eq!(resp.status(), 200, "创建配置应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建配置响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建配置业务码应为 200: {body}");
    assert_eq!(body["data"]["configKey"], key, "返回 configKey 应一致: {body}");
    assert_eq!(body["data"]["value"], "AdminSystem", "返回 value 应一致: {body}");

    // 按 key 回查确认落库
    let resp = client
        .get(format!("{}/api/v1/config/key/{key}", srv.base_url))
        .send()
        .await
        .expect("按 key 查询请求失败");
    let body: Value = resp.json().await.expect("按 key 查询响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "按 key 查询应成功: {body}");
    assert_eq!(body["data"]["value"], "AdminSystem", "按 key 查询应命中配置值: {body}");
}

/// 创建重复 key → 业务失败（非 200 业务码）
#[tokio::test]
async fn config_duplicate_key_rejected() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let key = unique_suffix("cfgdup");

    let make = || {
        json!({
            "category": "system",
            "configType": 0,
            "name": "重复",
            "configKey": key,
            "value": "v1",
        })
    };
    let resp = client
        .post(format!("{}/api/v1/config", srv.base_url))
        .json(&make())
        .send()
        .await
        .expect("首次创建请求失败");
    assert_eq!(resp.status(), 200, "首次创建应成功: {resp:?}");

    let resp = client
        .post(format!("{}/api/v1/config", srv.base_url))
        .json(&make())
        .send()
        .await
        .expect("重复创建请求失败");
    // 业务错误走 HTTP 200 + 错误业务码（10204 配置键已存在）
    assert_eq!(resp.status(), 200, "业务错误应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("重复创建响应 JSON 解析失败");
    let code = body["code"].as_i64().unwrap();
    assert_ne!(code, 200, "重复 key 不应返回成功业务码: {body}");
    assert_eq!(code, 10204, "重复 key 应返回业务码 10204: {body}");
}

/// 配置分页查询：按分类过滤命中新建配置
#[tokio::test]
async fn config_list_paged_filter_by_category() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let category = unique_suffix("cfgcat");

    for i in 0..3 {
        client
            .post(format!("{}/api/v1/config", srv.base_url))
            .json(&json!({
                "category": category,
                "configType": 0,
                "name": format!("配置-{i}"),
                "configKey": format!("{category}.k{i}"),
                "value": format!("v{i}"),
            }))
            .send()
            .await
            .expect("创建配置请求失败");
    }

    let resp = client
        .post(format!("{}/api/v1/config/list", srv.base_url))
        .json(&json!({ "category": category, "page": 1, "pageSize": 10 }))
        .send()
        .await
        .expect("分页查询请求失败");
    assert_eq!(resp.status(), 200, "分页查询应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("分页查询响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分页查询业务码应为 200: {body}");
    assert_eq!(body["data"]["total"], 3, "按分类过滤应命中 3 条: {body}");
    let list = body["data"]["list"].as_array().expect("list 应为数组");
    assert_eq!(list.len(), 3, "list 应包含 3 条: {body}");
}

// ═══════════════════════════════ Dict ═════════════════════════════════════

/// 创建字典类型 + 字典数据，按 code 查询命中
#[tokio::test]
async fn dict_create_type_and_data_by_code_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let dict_type = unique_suffix("dictt");

    // 创建字典类型
    let resp = client
        .post(format!("{}/api/v1/dict/type", srv.base_url))
        .json(&json!({ "name": "性别", "dictType": dict_type }))
        .send()
        .await
        .expect("创建字典类型请求失败");
    assert_eq!(resp.status(), 200, "创建字典类型应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建字典类型响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建字典类型业务码应为 200: {body}");
    assert_eq!(body["data"]["dictType"], dict_type, "返回 dictType 应一致: {body}");

    // 创建字典数据
    let resp = client
        .post(format!("{}/api/v1/dict/data", srv.base_url))
        .json(&json!({
            "sort": 1,
            "label": "男",
            "value": "1",
            "dictType": dict_type,
            "colorType": "success",
        }))
        .send()
        .await
        .expect("创建字典数据请求失败");
    assert_eq!(resp.status(), 200, "创建字典数据应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建字典数据响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建字典数据业务码应为 200: {body}");
    assert_eq!(body["data"]["value"], "1", "返回 value 应一致: {body}");
    assert_eq!(body["data"]["label"], "男", "返回 label 应一致: {body}");

    // 按 type 查询数据，验证落库关联
    let resp = client
        .get(format!("{}/api/v1/dict/data/type/{dict_type}", srv.base_url))
        .send()
        .await
        .expect("按 type 查询字典数据请求失败");
    let body: Value = resp.json().await.expect("按 type 查询响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "按 type 查询应成功: {body}");
    let items = body["data"].as_array().expect("字典数据应为数组");
    assert!(
        items.iter().any(|d| d["value"] == "1"),
        "按 type 查询应命中已创建数据: {body}"
    );
}

/// 字典数据分页查询：按 dictType 过滤命中
#[tokio::test]
async fn dict_data_list_filter_by_type() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let dict_type = unique_suffix("dictl");

    client
        .post(format!("{}/api/v1/dict/type", srv.base_url))
        .json(&json!({ "name": "类型", "dictType": dict_type }))
        .send()
        .await
        .expect("创建字典类型请求失败");

    for i in 0..2 {
        client
            .post(format!("{}/api/v1/dict/data", srv.base_url))
            .json(&json!({
                "sort": i + 1,
                "label": format!("值{i}"),
                "value": format!("{i}"),
                "dictType": dict_type,
            }))
            .send()
            .await
            .expect("创建字典数据请求失败");
    }

    let resp = client
        .post(format!("{}/api/v1/dict/data/list", srv.base_url))
        .json(&json!({ "dictType": dict_type, "page": 1, "pageSize": 10 }))
        .send()
        .await
        .expect("分页查询请求失败");
    assert_eq!(resp.status(), 200, "分页查询应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("分页查询响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "分页查询业务码应为 200: {body}");
    assert_eq!(body["data"]["total"], 2, "按 dictType 过滤应命中 2 条: {body}");
}

// ═══════════════════════════════ Log ══════════════════════════════════════

/// 创建操作日志 + 列表查询命中
#[tokio::test]
async fn log_operate_create_and_list_ok() {
    let srv = server().await;
    let client = authed_client(admin_token().await);
    let sub_type = unique_suffix("oplog");

    // 创建操作日志
    let resp = client
        .post(format!("{}/api/v1/log/operate", srv.base_url))
        .json(&json!({
            "traceId": "trace-1",
            "userId": "1",
            "userType": 1,
            "logType": "business",
            "subType": sub_type,
            "bizId": "1",
            "action": "创建操作日志",
            "success": 1,
            "extra": "{}",
        }))
        .send()
        .await
        .expect("创建操作日志请求失败");
    assert_eq!(resp.status(), 200, "创建操作日志应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("创建操作日志响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "创建操作日志业务码应为 200: {body}");

    // 列表查询：按 subType 过滤命中新建日志
    let resp = client
        .post(format!("{}/api/v1/log/operate/list", srv.base_url))
        .json(&json!({ "subType": sub_type, "page": 1, "pageSize": 10 }))
        .send()
        .await
        .expect("操作日志列表查询请求失败");
    assert_eq!(resp.status(), 200, "操作日志列表查询应返回 HTTP 200: {resp:?}");
    let body: Value = resp.json().await.expect("操作日志列表查询响应 JSON 解析失败");
    assert_eq!(body["code"], 200, "操作日志列表查询业务码应为 200: {body}");
    assert_eq!(body["data"]["total"], 1, "按 subType 过滤应命中 1 条: {body}");
    let list = body["data"]["list"].as_array().expect("list 应为数组");
    assert_eq!(list.len(), 1, "list 应包含 1 条: {body}");
    assert_eq!(list[0]["action"], "创建操作日志", "操作日志 action 应一致: {body}");
}

/// 未登录访问 Config/Dict/Log 接口 → HTTP 401（鉴权链路）
#[tokio::test]
async fn config_dict_log_api_requires_auth() {
    let srv = server().await;
    let client = reqwest::Client::new();

    let config_resp = client
        .get(format!("{}/api/v1/config/key/sys.name", srv.base_url))
        .send()
        .await
        .expect("config 请求失败");
    assert_eq!(config_resp.status(), 401, "未登录访问 config 应返回 401");

    let dict_resp = client
        .get(format!("{}/api/v1/dict/type/1", srv.base_url))
        .send()
        .await
        .expect("dict 请求失败");
    assert_eq!(dict_resp.status(), 401, "未登录访问 dict 应返回 401");

    let log_resp = client
        .post(format!("{}/api/v1/log/operate/list", srv.base_url))
        .json(&json!({ "page": 1, "pageSize": 10 }))
        .send()
        .await
        .expect("log 请求失败");
    assert_eq!(log_resp.status(), 401, "未登录访问 log 应返回 401");
}
