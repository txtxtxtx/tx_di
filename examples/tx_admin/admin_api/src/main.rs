mod plugin;
mod interfaces;

use tx_di_core::BuildContext;

#[allow(unused_imports)]
use tx_di_axum;
#[allow(unused_imports)]
use tx_di_log;
use tx_error::AppResult;

use interfaces::grpc::auth_service::AuthGrpcService;
use admin_proto::admin::auth::auth_service_server::AuthServiceServer;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config_path = r"D:\proj\tx_di\examples\tx_admin\config\config.toml";
    let app = BuildContext::new(Some(config_path)).build()?;
    let app = app.ins_run().await?;

    // 并行启动 gRPC 服务
    let grpc = tonic::transport::Server::builder()
        .add_service(AuthServiceServer::new(AuthGrpcService::default()));
    tokio::spawn(async move {
        let addr = "[::]:50051".parse().unwrap();
        tracing::info!("gRPC 服务器正在监听: {}", addr);
        if let Err(e) = grpc.serve(addr).await {
            tracing::error!("gRPC 服务器运行失败: {}", e);
        }
    });

    Ok(app.waiting_exit().await)
}
