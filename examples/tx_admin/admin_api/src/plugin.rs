use admin_app::log::app_service::OperateLogAppService;
use admin_proto::CreateOperateLogRequest;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tx_di_axum::{MetricsLayer, WebConfig, WebPlugin, add_layer};
use tx_di_core::{App, AppAllConfig, Component, DepsTuple, RIE};
use tx_di_sa_token::{SaCheckLoginLayer, SaTokenLayer, SaTokenPlugin};

use crate::interfaces::api;
use crate::interfaces::grpc;
use crate::operate_log::{OPERATE_LOG_CHANNEL_CAP, OperateLogEntry, OperateLogLayer};

use grpc::auth_service::AuthGrpcService;
use grpc::config_service::ConfigGrpcService;
use grpc::dept_service::DeptGrpcService;
use grpc::dict_service::DictGrpcService;
use grpc::file_service::FileGrpcService;
use grpc::job_service::{JobGrpcService, JobLogGrpcService};
use grpc::log_service::LogGrpcService;
use grpc::menu_service::MenuGrpcService;
use grpc::monitor_service::MonitorGrpcService;
use grpc::role_service::RoleGrpcService;
use grpc::tool_service::ToolGrpcService;
use grpc::user_service::UserGrpcService;

use admin_proto::admin::auth::auth_service_server::AuthServiceServer;
use admin_proto::admin::config::config_service_server::ConfigServiceServer;
use admin_proto::admin::dept::department_service_server::DepartmentServiceServer;
use admin_proto::admin::dict::dict_service_server::DictServiceServer;
use admin_proto::admin::file::file_service_server::FileServiceServer;
use admin_proto::admin::job::job_log_service_server::JobLogServiceServer;
use admin_proto::admin::job::job_service_server::JobServiceServer;
use admin_proto::admin::log::log_service_server::LogServiceServer;
use admin_proto::admin::menu::menu_service_server::MenuServiceServer;
use admin_proto::admin::monitor::monitor_service_server::MonitorServiceServer;
use admin_proto::admin::role::role_service_server::RoleServiceServer;
use admin_proto::admin::tool::tool_service_server::ToolServiceServer;
use admin_proto::admin::user::user_service_server::UserServiceServer;

/// gRPC 默认端口（可被 `GRPC_PORT` 环境变量或 `[grpc_config] port` 配置覆盖）
const DEFAULT_GRPC_PORT: u16 = 50051;

/// 解析 gRPC 监听端口，优先级：环境变量 `GRPC_PORT` > 配置 `[grpc_config].port` > 默认值
fn resolve_grpc_port(config: &AppAllConfig) -> u16 {
    std::env::var("GRPC_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .or_else(|| config.get::<u16>("grpc_config.port"))
        .unwrap_or(DEFAULT_GRPC_PORT)
}

#[derive(Component)]
#[component(app_async_init, init_sort = i32::MAX - 100)]
pub struct AdminPlugin;

/// `#[component(app_async_init)]` 回调：注册 HTTP 路由和 gRPC 服务
async fn app_async_init(_comp: Arc<AdminPlugin>, app: Arc<App>) -> RIE<()> {
    // 获取 sa-token 状态
    let sa_plugin = app.inject::<SaTokenPlugin>();
    let sa_state = sa_plugin.state().clone();

    // 获取 WebConfig 的 max_body_size，用于文件上传 Content-Length 提前拦截
    let web_config = app.inject::<WebConfig>();
    let max_body_size = web_config.max_body_size as u64;

    // 可观测性指标 Layer（阶段 E-3）：采集 HTTP 请求计数与耗时
    add_layer(MetricsLayer, 5); // sort=5: 在 api_log(10) 之前（指标应覆盖所有请求）
    info!("Prometheus 指标 Layer 已注册 (sort=5)");

    // 注册操作日志 Layer：每次 HTTP 请求自动写入 sys_operate_log 表
    let op_log_svc: Arc<OperateLogAppService> = app.inject();
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
                })
                .to_string(),
            };
            let _ = op_log_svc_clone.create_log(req).await;
        }
    });
    info!("操作日志 Layer 已注册 (sort=15)");

    // ════════════════════ 领域事件订阅（示例：事件驱动扩展点）════════════════
    // 领域事件由 AppService 在事务提交后发布（如 UserCreated）。
    // 此处为验证机制的示例订阅；生产场景可在此接入缓存失效、审计、通知等。
    {
        let event_bus = app.inject::<admin_app::event_bus::EventBus>();
        event_bus.on::<admin_domain::identity::user::model::event::UserEvent>(|event| {
            if let admin_domain::identity::user::model::event::UserEvent::UserCreated { user_id, username } =
                event
            {
                tracing::info!(
                    "[domain-event] 用户创建: id={} username={}",
                    user_id,
                    username
                );
            }
        });
        info!("领域事件总线订阅已注册");
    }

    // 构建路由：公开接口与受保护接口
    let open = api::open_router();
    let protected = api::router(max_body_size);

    let router = tx_di_axum::Router::new().merge(
        protected
            .layer(SaCheckLoginLayer::new())
            .layer(SaTokenLayer::new(sa_state)),
    );

    WebPlugin::add_router(open);
    WebPlugin::add_router(router);
    info!("admin HTTP 路由已注册（含认证）");

    // ════════════════════ gRPC Server ════════════════════

    let grpc_port = resolve_grpc_port(&app.inject::<AppAllConfig>());
    let grpc_addr: std::net::SocketAddr = format!("0.0.0.0:{}", grpc_port)
        .parse()
        .map_err(|e: std::net::AddrParseError| anyhow::anyhow!("gRPC 地址解析失败: {}", e))?;

    // ════════════════════ 注册中心端点注册（微服务化预留）════════════════
    // 将 HTTP/gRPC 端点声明到端点注册表，由 tx_di_nacos 的 app_loop! 在启动后
    // 统一收集（take_endpoints）并注册到 Nacos。生产通过 SERVICE_IP 环境变量指定注册 IP。
    {
        let http_port = web_config.port;
        tx_di_nacos::register_endpoints(std::sync::Arc::new(crate::nacos::AdminEndpoints::new(
            http_port, grpc_port,
        )));
        info!("已注册服务端点: HTTP={} gRPC={}", http_port, grpc_port);
    }

    // 使用 tower middleware 实现认证
    let auth_layer = grpc::auth_interceptor::AuthLayer::new();

    // 生产可观测性：gRPC 健康检查（tonic-health）与服务反射（tonic-reflection）。
    // - health：供 K8s gRPC 探针 / grpc_health_probe / 注册中心探活。
    // - reflection：供 grpcurl / grpcui 等工具动态发现服务与方法。
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    // 空服务名（""）表示整体健康状态，tonic-health 默认即为 Serving；
    // 这里显式再标记一次以保证语义清晰（grpc_health_probe 默认检查空服务名）。
    health_reporter
        .set_service_status("", tonic_health::ServingStatus::Serving)
        .await;
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(admin_proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .map_err(|e| anyhow::anyhow!("gRPC 反射服务构建失败: {e}"))?;

    // 构建 tonic Router，注册所有 gRPC 服务
    let grpc_router = tonic::transport::Server::builder()
        .layer(auth_layer)
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(AuthServiceServer::new(AuthGrpcService { app: app.clone() }))
        .add_service(UserServiceServer::new(UserGrpcService { app: app.clone() }))
        .add_service(RoleServiceServer::new(RoleGrpcService { app: app.clone() }))
        .add_service(MenuServiceServer::new(MenuGrpcService { app: app.clone() }))
        .add_service(DepartmentServiceServer::new(DeptGrpcService {
            app: app.clone(),
        }))
        .add_service(ConfigServiceServer::new(ConfigGrpcService {
            app: app.clone(),
        }))
        .add_service(DictServiceServer::new(DictGrpcService { app: app.clone() }))
        .add_service(LogServiceServer::new(LogGrpcService { app: app.clone() }))
        .add_service(FileServiceServer::new(FileGrpcService { app: app.clone() }))
        .add_service(MonitorServiceServer::new(MonitorGrpcService {
            app: app.clone(),
        }))
        .add_service(ToolServiceServer::new(ToolGrpcService))
        .add_service(JobServiceServer::new(JobGrpcService { app: app.clone() }))
        .add_service(JobLogServiceServer::new(JobLogGrpcService {
            app: app.clone(),
        }));

    tokio::spawn(async move {
        info!("gRPC server listening on {}", grpc_addr);
        if let Err(e) = grpc_router.serve(grpc_addr).await {
            tracing::error!("gRPC server error: {}", e);
        }
    });
    info!("admin gRPC 路由已注册（端口 {}）", grpc_port);

    Ok(())
}
