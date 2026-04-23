use std::sync::Arc;
use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::{info, error};
use tx_di_core::{tx_comp, BoxFuture, BuildContext, CompInit, RIE};
use crate::WebConfig;

/// Web 服务器插件组件
///
/// 提供基于 axum 的 web 服务器功能，支持依赖注入和配置管理。
/// 该组件会在 DI 框架初始化时自动注册并启动 web 服务器。
#[derive(Clone, Debug)]
#[tx_comp(init)]
pub struct WebPlugin {
    /// Web 服务器配置
    pub config: Arc<WebConfig>,

    /// Axum 路由器
    #[tx_cst(skip)]
    pub router: Router,
}

impl CompInit for WebPlugin {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        info!("Web 服务器插件初始化中...");
        self.router = self.router.clone()
            .route("/health", get(health_check));
        Ok(())
    }
    fn async_init(ctx: &mut BuildContext) -> BoxFuture<'static, RIE<()>> {
        let config = ctx.inject::<WebConfig>();
        let web = ctx.inject::<WebPlugin>();
        Box::pin(async move {
            start_server(config, web).await?;
            Ok(())
        })
    }

    /// 插件初始化排序，最后初始化
    fn init_sort() -> i32 {
        i32::MAX
    }
}

/// 健康检查端点
///
/// 返回简单的 OK 响应，用于负载均衡器或监控系统的健康检查
async fn health_check() -> &'static str {
    "OK"
}

/// 启动 web 服务器
///
/// # Arguments
///
/// * `config` - Web 服务器配置
/// * `router` - Axum 路由器
///
/// # Errors
///
/// 如果服务器绑定失败或运行出错，将返回错误
async fn start_server(config: Arc<WebConfig>, web: Arc<WebPlugin>) -> RIE<()> {
    let addr = config.socket_addr()?;
    
    info!("Web 服务器正在监听: {}", addr);
    info!("CORS 启用状态: {}", config.enable_cors);
    info!("最大请求体大小: {} bytes", config.max_body_size);
    info!("静态文件夹路径:{}",config.static_dir);

    let listener = TcpListener::bind(addr).await?;
    
    axum::serve(listener, web.router.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Web 服务器运行失败: {}", e))?;

    Ok(())
}
