//! 系统监控 HTTP API
//!
//! 服务器信息采用定时采集 + 缓存策略：后台任务每秒采集一次系统指标，
//! 存入固定长度的环形队列（默认 5 条），前端请求时直接读取缓存。

use std::sync::OnceLock;

use axum::extract::Query;
use axum::routing::get;
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, RingBuffer};
use serde::Deserialize;
use tokio::sync::RwLock;
use tx_common::ApiR;
use tx_di_axum::bound::DiComp;
use tx_di_axum::{R, Router};

use admin_app::user::app_service::UserAppService;
use admin_proto::{OnlineUser, OnlineUserListResponse, ServerInfo};
use tx_di_sa_token::StpUtil;

use crate::auth::ensure_permission;
use crate::error::ApiErr;

/// 服务器信息缓存，使用环形队列保存最近 N 条采集结果
static SERVER_CACHE: OnceLock<RwLock<HeapRb<ServerInfo>>> = OnceLock::new();

/// 缓存容量
const CACHE_CAPACITY: usize = 5;

/// 获取或初始化缓存，并启动后台采集任务
fn get_cache() -> &'static RwLock<HeapRb<ServerInfo>> {
    SERVER_CACHE.get_or_init(|| {
        // 启动后台采集任务
        tokio::spawn(async {
            loop {
                let info = collect_server_info();
                let cache = SERVER_CACHE.get().unwrap();
                let mut rb = cache.write().await;
                rb.push_overwrite(info); // 满时自动覆盖最旧元素
                drop(rb); // 释放锁后再 sleep
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        RwLock::new(HeapRb::new(CACHE_CAPACITY))
    })
}

/// 采集一次系统指标（同步，避免在 async 上下文中引入 System 的 !Send 问题）
fn collect_server_info() -> ServerInfo {
    use sysinfo::{Disks, System};

    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    // CPU 使用率需要两次采样，间隔 200ms
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_all();

    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
    let os_name = System::name().unwrap_or_else(|| "unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "unknown".to_string());

    let cpu_cores = sys.cpus().len() as u32;
    let cpu_usage = sys.global_cpu_usage() as f64;

    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let memory_usage = if total_memory > 0 {
        (used_memory as f64 / total_memory as f64) * 100.0
    } else {
        0.0
    };

    let disks = Disks::new_with_refreshed_list();
    let mut total_disk: u64 = 0;
    let mut used_disk: u64 = 0;
    for disk in &disks {
        total_disk += disk.total_space();
        used_disk += disk.total_space() - disk.available_space();
    }
    let disk_usage = if total_disk > 0 {
        (used_disk as f64 / total_disk as f64) * 100.0
    } else {
        0.0
    };

    ServerInfo {
        os_name,
        os_version,
        hostname,
        cpu_cores,
        cpu_usage,
        total_memory,
        used_memory,
        memory_usage,
        total_disk,
        used_disk,
        disk_usage,
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/server", get(get_server_info))
        .route("/online", get(get_online_users))
}

/// 查询参数
#[derive(Debug, Deserialize)]
struct ServerQuery {
    /// `true` 返回全部缓存记录，`false` 或缺省仅返回最新一条
    all: Option<bool>,
}

/// GET /api/monitor/server - 获取服务器信息
///
/// # 查询参数
/// * `all` - 可选，`true` 返回最近 5 条缓存记录，缺省或 `false` 仅返回最新一条
///
/// # 执行逻辑
/// 1. 从全局缓存队列中读取已采集的系统指标
/// 2. 根据 `all` 参数决定返回全部缓存还是仅最新一条
async fn get_server_info(
    Query(params): Query<ServerQuery>,
) -> Result<R<serde_json::Value>, ApiErr> {
    ensure_permission("system:view").await?;

    let cache = get_cache().read().await;

    if params.all.unwrap_or(false) {
        // 返回全部缓存
        let list: Vec<&ServerInfo> = cache.iter().collect();
        Ok(R(ApiR::success(serde_json::json!({ "list": list }))))
    } else {
        // 返回最新一条（环形队列迭代顺序为从旧到新，last 即最新）
        let latest = cache.iter().last().cloned().unwrap_or_else(|| ServerInfo {
            os_name: "unknown".to_string(),
            os_version: "unknown".to_string(),
            hostname: "unknown".to_string(),
            cpu_cores: 0,
            cpu_usage: 0.0,
            total_memory: 0,
            used_memory: 0,
            memory_usage: 0.0,
            total_disk: 0,
            used_disk: 0,
            disk_usage: 0.0,
        });
        Ok(R(ApiR::success(serde_json::json!(latest))))
    }
}

/// 分批查询大小
const ONLINE_BATCH_SIZE: i64 = 100;

/// GET /api/monitor/online - 获取在线用户列表
///
/// # 执行逻辑
/// 1. 先查询 Active 用户总数
/// 2. 分批（每批 100）遍历用户，通过 sa-token 检查会话是否活跃
/// 3. 对在线用户从 token extra_data 中提取登录 IP 和登录时间
async fn get_online_users(
    DiComp(user_svc): DiComp<UserAppService>,
) -> Result<R<OnlineUserListResponse>, ApiErr> {
    ensure_permission("system:view").await?;

    let status = Some(admin_domain::user::model::value_object::UserStatus::Active);
    let mut online_users: Vec<OnlineUser> = Vec::new();
    let mut page_num: i64 = 1;

    loop {
        let query = admin_app::user::dto::UserQueryRequest {
            username: None,
            nickname: None,
            mobile: None,
            status: status.clone(),
            dept_id: None,
            page: page_num,
            size: ONLINE_BATCH_SIZE,
        };
        let page_result = user_svc.get_user_page(query).await?;
        let is_last = page_result.list.len() < ONLINE_BATCH_SIZE as usize;

        for user in page_result.list {
            let user_id_str = user.id.to_string();
            // 检查该用户是否有活跃的 sa-token 会话
            if let Ok(token) = StpUtil::get_token_by_login_id(&user_id_str).await {
                if StpUtil::is_login(&token).await {
                    let (login_ip, login_time) = match StpUtil::get_token_info(&token).await {
                        Ok(info) => {
                            let ip = info
                                .extra_data
                                .as_ref()
                                .and_then(|d| d.get("login_ip"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("-")
                                .to_string();
                            let time = info.create_time.format("%Y-%m-%d %H:%M:%S").to_string();
                            (ip, time)
                        }
                        Err(_) => ("-".to_string(), "-".to_string()),
                    };

                    online_users.push(OnlineUser {
                        user_id: user.id,
                        username: user.username,
                        login_ip,
                        login_time,
                    });
                }
            }
        }

        if is_last {
            break;
        }
        page_num += 1;
    }

    let total = online_users.len() as u64;
    Ok(R(ApiR::success(OnlineUserListResponse {
        users: online_users,
        total,
    })))
}
