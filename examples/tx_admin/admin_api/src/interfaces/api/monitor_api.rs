//! 系统监控 HTTP API（示例 mock 数据）

use tx_di_axum::{R, Router};
use axum::routing::get;
use admin_proto::{ServerInfo, OnlineUser, OnlineUserListResponse};
use tx_common::ApiR;
use crate::auth::ensure_permission;
use crate::error::ApiErr;

pub fn router() -> Router {
    Router::new()
        .route("/server", get(get_server_info))
        .route("/online", get(get_online_users))
}

/// GET /api/monitor/server - 获取服务器信息
/// TODO: 当前返回硬编码的 mock 数据，应替换为真实的系统指标采集：
///   - 使用 sysinfo / sys-info 等 crate 获取真实的 CPU、内存、磁盘信息
///   - 考虑添加缓存，避免每次请求都采集系统指标（采集开销较大）
async fn get_server_info() -> Result<R<ServerInfo>, ApiErr> {
    ensure_permission("system:view").await?;
    Ok(R(ApiR::success(ServerInfo {
        os_name: "Linux".to_string(),
        os_version: "5.15.0-78-generic".to_string(),
        hostname: "tx-admin-server".to_string(),
        cpu_cores: 8,
        cpu_usage: 23.5,
        total_memory: 17_179_869_184,  // 16 GB
        used_memory: 8_589_934_592,    // 8 GB
        memory_usage: 50.0,
        total_disk: 512_000_000_000,   // 512 GB
        used_disk: 204_800_000_000,    // 200 GB
        disk_usage: 40.0,
    })))
}

/// GET /api/monitor/online - 获取在线用户列表
/// TODO: 当前返回硬编码的 mock 数据，应替换为真实的在线用户查询：
///   - 从 sa-token 会话存储中获取所有活跃会话（StpUtil::get_token_list）
///   - 或维护一个在线用户表/缓存，登录时写入、登出/过期时清除
async fn get_online_users() -> Result<R<OnlineUserListResponse>, ApiErr> {
    ensure_permission("system:view").await?;
    let users = vec![
        OnlineUser {
            user_id: 1,
            username: "admin".to_string(),
            login_ip: "192.168.1.100".to_string(),
            login_time: "2026-06-14 09:00:00".to_string(),
        },
        OnlineUser {
            user_id: 2,
            username: "zhangsan".to_string(),
            login_ip: "192.168.1.101".to_string(),
            login_time: "2026-06-14 10:30:00".to_string(),
        },
        OnlineUser {
            user_id: 3,
            username: "lisi".to_string(),
            login_ip: "10.0.0.55".to_string(),
            login_time: "2026-06-14 11:15:00".to_string(),
        },
    ];
    let total = users.len() as u64;
    Ok(R(ApiR::success(OnlineUserListResponse { users, total })))
}
