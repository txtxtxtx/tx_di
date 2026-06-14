//! 系统监控 HTTP API（示例 mock 数据）

use tx_di_axum::{R, Router};
use tx_di_axum::aide::axum::routing::get;
use admin_proto::{ServerInfo, OnlineUser, OnlineUserListResponse};
use tx_common::ApiR;

pub fn router() -> Router {
    Router::new()
        .api_route("/server", get(get_server_info))
        .api_route("/online", get(get_online_users))
}

/// GET /api/monitor/server - 获取服务器信息
async fn get_server_info() -> R<ServerInfo> {
    R(ApiR::success(ServerInfo {
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
    }))
}

/// GET /api/monitor/online - 获取在线用户列表
async fn get_online_users() -> R<OnlineUserListResponse> {
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
    R(ApiR::success(OnlineUserListResponse { users, total }))
}
