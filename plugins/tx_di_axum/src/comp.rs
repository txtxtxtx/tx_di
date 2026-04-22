use std::sync::Arc;
use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::{info, error};
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};
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
    fn inner_init(&mut self, ctx: &mut BuildContext) -> RIE<()> {
        info!("Web 服务器插件初始化中...");

        // 添加一个默认的健康检查路由
        self.router = self.router.clone()
            .route("/health", get(health_check));

        // 在后台启动服务器
        let config = self.config.clone();
        let router = self.router.clone();

        tokio::spawn(async move {
            match start_server(config, router).await {
                Ok(_) => info!("Web 服务器已停止"),
                Err(e) => error!("Web 服务器运行出错: {}", e),
            }
        });

        info!("Web 服务器插件初始化完成");
        Ok(())
    }

    /// 插件初始化排序，在日志之后初始化
    fn init_sort() -> i32 {
        i32::MIN + 100
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
async fn start_server(config: Arc<WebConfig>, router: Router) -> anyhow::Result<()> {
    let addr = config.socket_addr()?;
    
    info!("Web 服务器正在监听: {}", addr);
    info!("CORS 启用状态: {}", config.enable_cors);
    info!("最大请求体大小: {} bytes", config.max_body_size);

    let listener = TcpListener::bind(addr).await?;
    
    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("Web 服务器运行失败: {}", e))?;

    Ok(())
}
