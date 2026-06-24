use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tx_di_axum::{WebPlugin, WebConfig, add_layer};
use tx_di_core::{App, CancellationToken, CompInit, RIE, async_method, tx_comp};
use tx_di_sa_token::{SaTokenPlugin, SaTokenLayer, SaCheckLoginLayer};
use admin_app::log::app_service::OperateLogAppService;
use admin_proto::CreateOperateLogRequest;

use crate::interfaces::api;
use crate::interfaces::grpc;
use crate::operate_log::{OperateLogLayer, OperateLogEntry, OPERATE_LOG_CHANNEL_CAP};

use grpc::auth_service::AuthGrpcService;
use grpc::user_service::UserGrpcService;
use grpc::role_service::RoleGrpcService;
use grpc::menu_service::MenuGrpcService;
use grpc::dept_service::DeptGrpcService;
use grpc::config_service::ConfigGrpcService;
use grpc::dict_service::DictGrpcService;
use grpc::log_service::LogGrpcService;
use grpc::file_service::FileGrpcService;
use grpc::monitor_service::MonitorGrpcService;
use grpc::tool_service::ToolGrpcService;
use grpc::job_service::{JobGrpcService, JobLogGrpcService};

use admin_proto::admin::auth::auth_service_server::AuthServiceServer;
use admin_proto::admin::user::user_service_server::UserServiceServer;
use admin_proto::admin::role::role_service_server::RoleServiceServer;
use admin_proto::admin::menu::menu_service_server::MenuServiceServer;
use admin_proto::admin::dept::department_service_server::DepartmentServiceServer;
use admin_proto::admin::config::config_service_server::ConfigServiceServer;
use admin_proto::admin::dict::dict_service_server::DictServiceServer;
use admin_proto::admin::log::log_service_server::LogServiceServer;
use admin_proto::admin::file::file_service_server::FileServiceServer;
use admin_proto::admin::monitor::monitor_service_server::MonitorServiceServer;
use admin_proto::admin::tool::tool_service_server::ToolServiceServer;
use admin_proto::admin::job::job_service_server::JobServiceServer;
use admin_proto::admin::job::job_log_service_server::JobLogServiceServer;

/// gRPC 默认端口
const DEFAULT_GRPC_PORT: u16 = 50051;

#[tx_comp(init)]
pub struct AdminPlugin;

impl CompInit for AdminPlugin {
    async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            // 获取 sa-token 状态
            let sa_plugin = ctx.inject::<SaTokenPlugin>();
            let sa_state = sa_plugin.state().clone();

            // 获取 WebConfig 的 max_body_size，用于文件上传 Content-Length 提前拦截
            let web_config = ctx.inject::<WebConfig>();
            let max_body_size = web_config.max_body_size as u64;

            // 注册操作日志 Layer：每次 HTTP 请求自动写入 sys_operate_log 表
            let op_log_svc: Arc<OperateLogAppService> = ctx.inject();
            let (op_log_tx, mut op_log_rx) = mpsc::channel::<OperateLogEntry>(OPERATE_LOG_CHANNEL_CAP);
            let op_log_layer = OperateLogLayer::new(op_log_tx);
            add_layer(op_log_layer, 15); // sort=15: 紧接 api_log(10) 之后

            let op_log_svc_clone = op_log_svc.clone();
            tokio::spawn(async move {
                while let Some(entry) = op_log_rx.recv().await {
                    let user_id = entry.user_id.unwrap_or(0);
                    let user_name = entry.user_name.unwrap_or_default();
                    let tenant_id = entry.tenant_id.unwrap_or(0);
                    let req = CreateOperateLogRequest {
                        trace_id: String::new(),
                        user_id,
                        user_type: if user_id > 0 { 1 } else { 0 },
                        log_type: "http".to_string(),
                        sub_type: entry.method,
                        biz_id: tenant_id,
                        action: entry.uri,
                        success: if entry.status < 400 { 1 } else { 0 },
                        extra: serde_json::json!({
                            "status": entry.status,
                            "latency_ms": format!("{:.2}", entry.latency_ms),
                            "user_ip": entry.user_ip,
                            "user_name": user_name,
                            "user_agent": entry.user_agent,
                        }).to_string(),
                    };
                    let _ = op_log_svc_clone.create_log(req).await;
                }
            });
            info!("操作日志 Layer 已注册 (sort=15)");

            // 构建路由：公开接口与受保护接口
            let open = api::open_router();
            let protected = api::router(max_body_size);

            let router = tx_di_axum::Router::new()
                .merge(open)
                .merge(
                    protected
                        .layer(SaCheckLoginLayer::new())
                        .layer(SaTokenLayer::new(sa_state))
                );

            WebPlugin::add_router(router);
            info!("admin HTTP 路由已注册（含认证）");

            // ════════════════════ gRPC Server ════════════════════

            let grpc_port = DEFAULT_GRPC_PORT;
            let grpc_addr: std::net::SocketAddr = format!("0.0.0.0:{}", grpc_port).parse()
                .map_err(|e: std::net::AddrParseError| anyhow::anyhow!("gRPC 地址解析失败: {}", e))?;

            // 使用 tower middleware 实现认证
            let auth_layer = grpc::auth_interceptor::AuthLayer::new();

            // 构建 tonic Router，注册所有 gRPC 服务
            let grpc_router = tonic::transport::Server::builder()
                .layer(auth_layer)
                .add_service(AuthServiceServer::new(AuthGrpcService { app: ctx.clone() }))
                .add_service(UserServiceServer::new(UserGrpcService { app: ctx.clone() }))
                .add_service(RoleServiceServer::new(RoleGrpcService { app: ctx.clone() }))
                .add_service(MenuServiceServer::new(MenuGrpcService { app: ctx.clone() }))
                .add_service(DepartmentServiceServer::new(DeptGrpcService { app: ctx.clone() }))
                .add_service(ConfigServiceServer::new(ConfigGrpcService { app: ctx.clone() }))
                .add_service(DictServiceServer::new(DictGrpcService { app: ctx.clone() }))
                .add_service(LogServiceServer::new(LogGrpcService { app: ctx.clone() }))
                .add_service(FileServiceServer::new(FileGrpcService { app: ctx.clone() }))
                .add_service(MonitorServiceServer::new(MonitorGrpcService { app: ctx.clone() }))
                .add_service(ToolServiceServer::new(ToolGrpcService))
                .add_service(JobServiceServer::new(JobGrpcService { app: ctx.clone() }))
                .add_service(JobLogServiceServer::new(JobLogGrpcService { app: ctx.clone() }));

            tokio::spawn(async move {
                info!("gRPC server listening on {}", grpc_addr);
                if let Err(e) = grpc_router.serve(grpc_addr).await {
                    tracing::error!("gRPC server error: {}", e);
                }
            });
            info!("admin gRPC 路由已注册（端口 {}）", grpc_port);

            Ok(())
        }
    );
    fn init_sort() -> i32 {
        // 在 DbInitPlugin 之后初始化（确保数据库已连接且数据已初始化）
        i32::MAX - 100
    }
}
