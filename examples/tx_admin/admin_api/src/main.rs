mod plugin;
mod interfaces;
mod services;

use tx_di_core::BuildContext;

#[allow(unused_imports)]
use tx_di_axum;
#[allow(unused_imports)]
use tx_di_log;
use tx_error::AppResult;

use interfaces::grpc::{
    auth_service::AuthGrpcService,
    user_service::UserGrpcService,
    role_service::RoleGrpcService,
    menu_service::MenuGrpcService,
    dept_service::DeptGrpcService,
    permission_service::PermissionGrpcService,
    config_service::ConfigGrpcService,
    dict_service::DictGrpcService,
    log_service::LogGrpcService,
    file_service::FileGrpcService,
};

use admin_proto::admin::auth::auth_service_server::AuthServiceServer;
use admin_proto::admin::user::user_service_server::UserServiceServer;
use admin_proto::admin::role::role_service_server::RoleServiceServer;
use admin_proto::admin::menu::menu_service_server::MenuServiceServer;
use admin_proto::admin::dept::department_service_server::DepartmentServiceServer;
use admin_proto::admin::permission::permission_service_server::PermissionServiceServer;
use admin_proto::admin::config::config_service_server::ConfigServiceServer;
use admin_proto::admin::dict::dict_service_server::DictServiceServer;
use admin_proto::admin::log::log_service_server::LogServiceServer;
use admin_proto::admin::file::file_service_server::FileServiceServer;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
    let app = BuildContext::new(Some(config_path)).build()?;
    let app = app.ins_run().await?;

    // 并行启动 gRPC 服务
    // let grpc = tonic::transport::Server::builder()
    //     .add_service(AuthServiceServer::new(AuthGrpcService::default()))
    //     .add_service(UserServiceServer::new(UserGrpcService::default()))
    //     .add_service(RoleServiceServer::new(RoleGrpcService::default()))
    //     .add_service(MenuServiceServer::new(MenuGrpcService::default()))
    //     .add_service(DepartmentServiceServer::new(DeptGrpcService::default()))
    //     .add_service(PermissionServiceServer::new(PermissionGrpcService::default()))
    //     .add_service(ConfigServiceServer::new(ConfigGrpcService::default()))
    //     .add_service(DictServiceServer::new(DictGrpcService::default()))
    //     .add_service(LogServiceServer::new(LogGrpcService::default()))
    //     .add_service(FileServiceServer::new(FileGrpcService::default()));
    // tokio::spawn(async move {
    //     let addr = "[::]:50051".parse().unwrap();
    //     tracing::info!("gRPC 服务器正在监听: {}", addr);
    //     if let Err(e) = grpc.serve(addr).await {
    //         tracing::error!("gRPC 服务器运行失败: {}", e);
    //     }
    // });

    Ok(app.waiting_exit().await)
}
