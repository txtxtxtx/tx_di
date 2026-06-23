//! 系统监控 gRPC 服务实现
//!
//! 服务器信息采集逻辑复用 HTTP 层的 monitor_api 缓存机制。

use std::sync::Arc;
use tonic::{Request, Response, Status};
use ringbuf::traits::Consumer;

use admin_proto::admin::monitor::monitor_service_server::MonitorService;
use admin_proto::admin::monitor::{OnlineUser, OnlineUserListResponse, ServerInfo};
use tx_di_core::App;
use tx_di_sa_token::StpUtil;

use super::auth_interceptor::{self, get_login_id};

#[derive(Clone)]
pub struct MonitorGrpcService {
    pub app: Arc<App>,
}

#[tonic::async_trait]
impl MonitorService for MonitorGrpcService {
    async fn get_server_info(
        &self,
        request: Request<()>,
    ) -> Result<Response<ServerInfo>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "system:view").await?;

        // 复用 HTTP 层的缓存采集逻辑
        let cache = crate::interfaces::api::monitor_api::get_cache().read().await;
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
            disks: vec![],
            networks: vec![],
        });
        Ok(Response::new(latest))
    }

    async fn get_online_users(
        &self,
        request: Request<()>,
    ) -> Result<Response<OnlineUserListResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "system:view").await?;

        let user_svc: Arc<admin_app::user::app_service::UserAppService> = self.app.inject();

        let status = Some(admin_domain::user::model::value_object::UserStatus::Active);
        let mut online_users = Vec::new();
        let mut page_num: i64 = 1;
        let batch_size: i64 = 100;

        loop {
            let req = admin_proto::ListUsersRequest {
                username: None,
                nickname: None,
                mobile: None,
                status: status.map(|s| s as i32),
                dept_id: None,
                page_info: Some(admin_proto::PageRequest {
                    page: page_num,
                    size: batch_size,
                }),
            };
            let page_result = user_svc
                .get_user_page(req)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            let is_last = page_result.list.len() < batch_size as usize;

            for user in page_result.list {
                let user_id_str = user.id.to_string();
                if let Ok(token) = StpUtil::get_token_by_login_id(&user_id_str).await {
                    if StpUtil::is_login(&token).await {
                        let (ip, time) = match StpUtil::get_token_info(&token).await {
                            Ok(info) => {
                                let ip = info
                                    .extra_data
                                    .as_ref()
                                    .and_then(|d| d.get("login_ip"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("-")
                                    .to_string();
                                let time =
                                    info.create_time.format("%Y-%m-%d %H:%M:%S").to_string();
                                (ip, time)
                            }
                            Err(_) => ("-".to_string(), "-".to_string()),
                        };
                        online_users.push(OnlineUser {
                            user_id: user.id,
                            username: user.username,
                            login_ip: ip,
                            login_time: time,
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
        Ok(Response::new(OnlineUserListResponse {
            users: online_users,
            total,
        }))
    }
}
