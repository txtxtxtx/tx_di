use std::net::{SocketAddr, TcpListener};
use crate::bound::AppStatus;
use crate::{WebConfig, R};
use axum::http::{Request};
use axum::{routing::get, Router};
use std::sync::{Arc, LazyLock, OnceLock, RwLock};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::TcpListener as TokioTcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use tx_di_core::{tx_comp, ApiR, App, CompInit, FormattedDateTime, RIE};
use crate::layers::{add_static_path_prefix, freeze_static_path_prefixes, LAYER_REGISTRY};

/// 全局路由器注册表
///
/// 用于在应用启动前收集所有模块注册的路由器
static ROUTER_REGISTRY: LazyLock<Arc<RwLock<Vec<Router>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// API 路由器注册表（带 OpenAPI 文档）
///
/// 仅在启用 `api-doc` feature 时可用，用于收集需要生成接口文档的路由器
#[cfg(feature = "api-doc")]
static API_ROUTER_REGISTRY: LazyLock<Arc<RwLock<Vec<aide::axum::ApiRouter>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// Web 服务器插件组件
///
/// 提供基于 axum 的 web 服务器功能，支持依赖注入和配置管理。
/// 该组件会在 DI 框架初始化时自动注册并启动 web 服务器。
// #[derive(Clone, Debug)]
#[tx_comp(init)]
pub struct WebPlugin {
    /// Web 服务器配置
    pub config: Arc<WebConfig>,

    /// Axum 路由器
    #[tx_cst(OnceLock::new())]
    pub router: OnceLock<Router>,
}

impl CompInit for WebPlugin {
    fn async_init_impl(ctx: Arc<App>,_token: CancellationToken) -> impl Future<Output = RIE<()>> + Send {
        let config = ctx.inject::<WebConfig>();
        let web = ctx.inject::<WebPlugin>();
        async move {
            let router = WebPlugin::merge_routers();
            let app_status = AppStatus { app: ctx.clone() };
            let mut router = router
                // todo 这里可以全局注入 DB
                // 吧 app 注入到 extensions
                .layer(tower::ServiceBuilder::new()
                    .map_request(move |mut req: Request<_>| {
                        req.extensions_mut().insert(app_status.clone());
                        req
                    })
                    .into_inner());
            // 如果配置了静态文件目录，添加静态文件服务
            let static_dir = config.static_dir();
            if static_dir.exists() {
                info!("静态文件目录已配置: {:?}", static_dir);
                router = router.nest_service(
                    "/static",
                    tower_http::services::ServeDir::new(&static_dir)
                        .precompressed_gzip()
                        .precompressed_br()
                );
            } else {
                debug!("静态文件目录不存在，跳过静态文件服务: {:?}", static_dir);
            }
            router = WebPlugin::setup_spa_apps(router, &config);

            // [api-doc] 注册 OpenAPI 文档端点
            #[cfg(feature = "api-doc")]
            {
                router = WebPlugin::setup_api_doc(router, &config);
            }

            // 添加中间件
            router = WebPlugin::layer_with_router(router);
            web.router.set(router).map_err(|_| "已经设置过路由了")?;
            Ok(())
        }
    }

    fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> impl Future<Output = RIE<()>> + Send {
        let config = ctx.inject::<WebConfig>();
        let router = ctx.inject::<WebPlugin>().router.get().unwrap().clone();
        async move {
            start_server(config, router,token).await?;
            Ok(())
        }
    }
    /// 插件初始化排序，最后初始化
    fn init_sort() -> i32 {
        i32::MAX
    }
}


impl WebPlugin {
    /// 配置静态文件服务（支持 SPA 和多前端项目）
    fn setup_spa_apps(mut router: Router, config: &WebConfig) -> Router {
        // 如果配置了 spa_apps，使用多前端项目模式
        if let Some(spa_apps) = &config.spa_apps {
            info!("检测到 {} 个 SPA 应用配置", spa_apps.len());
            for (path_prefix, dist_dir) in spa_apps {
                let dist_path = std::path::PathBuf::from(dist_dir);
                if dist_path.exists() {
                    info!("注册 SPA 应用: {} -> {:?}", path_prefix, dist_path);
                    // 创建 fallback 服务，用于 SPA 路由
                    let fallback = tower_http::services::ServeFile::new(dist_path.join("index.html"));

                    router = router.nest_service(
                        path_prefix,
                        tower_http::services::ServeDir::new(&dist_path)
                            .precompressed_gzip()
                            .precompressed_br()
                            .fallback(fallback)
                    );
                    add_static_path_prefix(path_prefix)
                } else {
                    error!("SPA 应用目录不存在: {:?}，已跳过", dist_path);
                }
            }
            freeze_static_path_prefixes();
        }
        router
    }

    /// 添加路由器
    ///
    /// 该方法用于添加一个新的路由器，该路由器将合并到 Web 插件的路由器中。
    /// ## **必须在上下文创建前添加路由器，否则路由无效**
    ///
    /// # Arguments
    ///
    /// * `router` - 要添加的路由器
    pub fn add_router(router: Router) {
        if let Ok(mut routers) = ROUTER_REGISTRY.write() {
            routers.push(router);
            info!("路由器已注册到全局注册表");
        } else {
            error!("无法获取路由器注册表的写锁");
        }
    }

    /// 添加带 OpenAPI 文档的路由器
    ///
    /// 使用 `aide::axum::ApiRouter` 注册路由，启动后自动生成接口文档。
    /// 访问 `/docs` 查看 Redoc 文档，`/api-docs/openapi.json` 获取 OpenAPI spec。
    ///
    /// ## **必须在上下文创建前添加路由器，否则路由无效**
    ///
    /// # Arguments
    ///
    /// * `router` - 要添加的 API 路由器
    #[cfg(feature = "api-doc")]
    pub fn add_api_router(router: aide::axum::ApiRouter) {
        if let Ok(mut routers) = API_ROUTER_REGISTRY.write() {
            routers.push(router);
            info!("API 路由器已注册到全局注册表（带 OpenAPI 文档）");
        } else {
            error!("无法获取 API 路由器注册表的写锁");
        }
    }

    /// 合并所有已注册的路由器
    ///
    /// 将全局注册表中的所有普通路由器合并为一个主路由器。
    /// API 路由器的合并由 `setup_api_doc` 负责。
    fn merge_routers() -> Router {
        let mut main_router = Router::new()
            .route("/health", get(health_check))
            ;

        // 合并所有普通路由器
        if let Ok(routers) = ROUTER_REGISTRY.read() {
            for router in routers.iter() {
                main_router = main_router.merge(router.clone());
            }
            info!("已合并 {} 个路由器", routers.len());
        } else {
            error!("无法获取路由器注册表的读锁");
        }

        main_router
    }

    /// 注册 API 文档端点（受 `enable_api_doc` 配置控制）
    ///
    /// 从 `API_ROUTER_REGISTRY` 中取出所有 `ApiRouter`，合并后通过 `finish_api`
    /// 同时生成 OpenAPI spec 和 axum Router，然后注册文档端点。
    ///
    /// 当 `enable_api_doc = true` 时，注册：
    /// - `/api-docs/openapi.json` — OpenAPI spec JSON
    /// - `/docs` — Redoc 文档页面（JS 内嵌，无需外部文件）
    #[cfg(feature = "api-doc")]
    fn setup_api_doc(mut router: Router, config: &WebConfig) -> Router {
        use aide::axum::ApiRouter;
        use aide::openapi::OpenApi;
        use aide::redoc::Redoc;

        if !config.enable_api_doc {
            info!("API 文档已禁用（enable_api_doc = false）");
            return router;
        }

        let mut api = OpenApi::default();
        let api_count;

        // 合并所有 API 路由器
        {
            let routers = API_ROUTER_REGISTRY.read();
            match routers {
                Ok(routers) if !routers.is_empty() => {
                    let mut api_router = ApiRouter::new();
                    for r in routers.iter() {
                        api_router = api_router.merge(r.clone());
                    }
                    // finish_api 消耗 ApiRouter，同时产出 Router + OpenAPI spec
                    let api_routes = api_router.finish_api(&mut api);
                    router = router.merge(api_routes);
                    api_count = routers.len();
                }
                _ => {
                    debug!("未注册任何 API 路由器，跳过 OpenAPI 文档生成");
                    return router;
                }
            }
        }

        // 注册 OpenAPI JSON 端点
        match serde_json::to_value(&api) {
            Ok(spec) => {
                router = router.route(
                    "/api-docs/openapi.json",
                    get(move || async { axum::response::Json(spec) }),
                );
            }
            Err(e) => {
                error!("序列化 OpenAPI spec 失败: {}", e);
                return router;
            }
        }

        // 注册 Redoc 文档页面（JS 内嵌在 aide crate 中，无外部依赖）
        // 泄漏 HTML 字符串为 'static，服务整个进程生命周期
        let redoc_html: &'static str = Box::leak(
            Redoc::new("/api-docs/openapi.json")
                .with_title("API Documentation")
                .html()
                .into_boxed_str()
        );
        router = router.route("/docs", get(move || async move { axum::response::Html(redoc_html) }));

        info!("已合并 {} 个 API 路由器，已注册 /docs 和 /api-docs/openapi.json", api_count);

        router
    }

    /// 应用所有中间件层
    pub fn layer_with_router(router: Router) -> Router {
        let mut router = router;
        // 应用所有中间件层（按优先级排序）sort 大的在外层，sort 小的在内层
        if let Ok(layers) = LAYER_REGISTRY.read() {
            let mut sorted_layers: Vec<_> = layers.iter().collect();
            sorted_layers.sort_by_key(|(sort, _)| *sort);
            let mut names = Vec::with_capacity(sorted_layers.len());
            for (_, middleware) in &sorted_layers {
                router = middleware.apply_to_router(router);
                names.push(middleware.name());
            }
            info!("已应用 {} 个中间件层（已按优先级排序）:[{}]", sorted_layers.len(),names.join(", "));
        } else {
            error!("无法获取中间件注册表的读锁");
        }
        router
    }


    /// 清空所有已注册的路由器
    ///
    /// 主要用于测试场景
    pub fn clear_routers() {
        if let Ok(mut routers) = ROUTER_REGISTRY.write() {
            routers.clear();
            info!("已清空所有注册的路由器");
        }
    }

    /// 清空所有已注册的中间件层
    ///
    /// 主要用于测试场景
    pub fn clear_layers() {
        if let Ok(mut layers) = LAYER_REGISTRY.write() {
            layers.clear();
            info!("已清空所有注册的中间件层");
        }
    }

    /// 获取所有已注册的路由器
    pub fn get_main_router() -> Vec<Router> {
        if let Ok(routers) = ROUTER_REGISTRY.read(){
            routers.clone()
        }else {
            vec![]
        }
    }
}

/// 创建一个 TCP 监听器,支持IPv4 和 IPv6
fn create_tcp_listener(addr: SocketAddr) -> RIE<TokioTcpListener> {
    // 根据地址类型选择 socket 域
    let domain = if addr.is_ipv6() {
        Domain::IPV6
    } else {
        Domain::IPV4
    };
    // 创建一个 IPv6 socket
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

    // 仅在 IPv6 时禁用 IPV6_V6ONLY，使 IPv6 socket 也能接受 IPv4 连接（双栈）
    if addr.is_ipv6() {
        socket.set_only_v6(false)?;
    }

    // 允许地址重用（可选，便于重启）
    socket.set_reuse_address(true)?;

    #[cfg(unix)]
    socket.set_reuse_port(true)?; // Windows 不支持 SO_REUSEPORT

    // 绑定地址
    socket.bind(&addr.into())?;

    // 设置监听队列长度（以系统的最大值限制）
    socket.listen(65536)?;

    // 设置为非阻塞
    socket.set_nonblocking(true)?;

    // 转换为标准库的 TcpListener
    let listener: TcpListener = socket.into();

    // 转换为 Tokio 的 TcpListener
    let tokio_listener = TokioTcpListener::from_std(listener)?;

    Ok(tokio_listener)
}

/// 健康检查端点
///
/// 返回简单的 OK 响应，用于负载均衡器或监控系统的健康检查
async fn health_check() -> R<FormattedDateTime> {
    ApiR::success(FormattedDateTime::now()).into()
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
async fn start_server(config: Arc<WebConfig>, router: Router,token: CancellationToken) -> RIE<()> {
    let addr = config.socket_addr()?;

    info!("CORS 启用状态: {}", config.enable_cors);
    info!("最大请求体大小: {} bytes", config.max_body_size);
    info!("静态文件夹路径:{}",config.static_dir);
    info!("Web 服务器正在监听: {}", addr);
    let listener = create_tcp_listener(addr)?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            token.cancelled().await;
        })
        .await
        .map_err(|e| anyhow::anyhow!("Web 服务器运行失败: {}", e))?;

    Ok(())
}
