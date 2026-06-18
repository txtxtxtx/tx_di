use std::net::{SocketAddr, TcpListener};
use crate::bound::AppStatus;
use crate::{WebConfig};
use axum::http::Request;
use axum::routing::get;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::TcpListener as TokioTcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use tx_di_core::{tx_comp, ApiR, App, CompInit, FormattedDateTime, RIE};
use crate::layers::LAYER_REGISTRY;

/// 全局路由器注册表
///
/// 使用 `Mutex<Vec>` 存储，合并时 drain 取出（`ApiRouter` 不实现 `Clone`）
static ROUTER_REGISTRY: LazyLock<Mutex<Vec<crate::Router>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// 生成的 OpenAPI spec（JSON Value），供文档端点使用
#[cfg(feature = "api-doc")]
static OPENAPI_SPEC: OnceLock<serde_json::Value> = OnceLock::new();

/// Web 服务器插件组件
///
/// 提供基于 axum 的 web 服务器功能，支持依赖注入和配置管理。
/// 该组件会在 DI 框架初始化时自动注册并启动 web 服务器。
#[tx_comp(init)]
pub struct WebPlugin {
    /// Web 服务器配置
    pub config: Arc<WebConfig>,

    /// Axum 路由器（最终合并后的标准 Router，用于 axum::serve）
    #[tx_cst(OnceLock::new())]
    pub router: OnceLock<axum::Router>,
}

impl CompInit for WebPlugin {
    fn async_init_impl(ctx: Arc<App>,_token: CancellationToken) -> impl Future<Output = RIE<()>> + Send {
        let config = ctx.inject::<WebConfig>();
        let web = ctx.inject::<WebPlugin>();
        async move {
            // ═══ 中间件内的路由（API 接口） ═══
            let mut api_router = WebPlugin::merge_routers();
            let app_status = AppStatus { app: ctx.clone() };
            api_router = api_router
                .layer(tower::ServiceBuilder::new()
                    .map_request(move |mut req: Request<_>| {
                        req.extensions_mut().insert(app_status.clone());
                        req
                    })
                    .into_inner());
            // 应用中间件（日志、CORS、超时等）
            api_router = WebPlugin::layer_with_router(api_router);

            // ═══ 中间件外的路由（静态资源、文档） ═══
            let mut outer_router = axum::Router::new();

            // 静态文件服务
            let static_dir = config.static_dir();
            if static_dir.exists() {
                info!("静态文件目录已配置: {:?}", static_dir);
                outer_router = outer_router.nest_service(
                    "/static",
                    tower_http::services::ServeDir::new(&static_dir)
                        .precompressed_gzip()
                        .precompressed_br()
                );
            } else {
                debug!("静态文件目录不存在，跳过静态文件服务: {:?}", static_dir);
            }

            // SPA 应用
            outer_router = WebPlugin::setup_spa_apps(outer_router, &config);

            // API 文档端点（/docs, /api-docs/openapi.json）
            outer_router = WebPlugin::setup_doc_routes(outer_router);

            // 合并：中间件外的路由优先匹配
            let router = outer_router.merge(api_router);

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
    fn setup_spa_apps(mut router: axum::Router, config: &WebConfig) -> axum::Router {
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
                } else {
                    error!("SPA 应用目录不存在: {:?}，已跳过", dist_path);
                }
            }
        }
        router
    }

    /// 添加路由器
    ///
    /// 使用 [`crate::Router`] 注册路由，该类型根据 `api-doc` feature 自动切换：
    /// - `api-doc` 启用时：`Router` = `ApiRouter`，自动生成 OpenAPI 文档
    /// - `api-doc` 禁用时：`Router` = `axum::Router`
    ///
    /// ## **必须在上下文创建前添加路由器，否则路由无效**
    ///
    /// # Arguments
    ///
    /// * `router` - 要添加的路由器
    pub fn add_router(router: crate::Router) {
        if let Ok(mut routers) = ROUTER_REGISTRY.lock() {
            routers.push(router);
            info!("路由器已注册到全局注册表");
        } else {
            error!("无法获取路由器注册表的锁");
        }
    }

    /// 添加原始 axum 路由器（不参与 OpenAPI 文档生成）
    ///
    /// 当 `api-doc` 启用时，接受 `axum::Router` 并包装为 `ApiRouter`。
    /// 适用于不需要文档的路由（如内部管理接口、第三方库提供的路由）。
    ///
    /// ## **必须在上下文创建前添加路由器，否则路由无效**
    pub fn add_axum_router(router: axum::Router) {
        #[cfg(feature = "api-doc")]
        {
            use aide::axum::ApiRouter;
            let api_router = ApiRouter::new().merge(router);
            Self::add_router(api_router);
        }
        #[cfg(not(feature = "api-doc"))]
        {
            Self::add_router(router);
        }
    }

    /// 合并所有已注册的路由器
    ///
    /// 返回 `(Router, Option<OpenApi>)`，OpenAPI spec 用于后续注册文档端点
    #[cfg(feature = "api-doc")]
    fn merge_routers() -> axum::Router {
        use aide::openapi::OpenApi;

        let mut main_router = axum::Router::new()
            .route("/health", get(health_check));

        let mut routers = ROUTER_REGISTRY.lock();
        if let Ok(ref mut routers) = routers {
            if !routers.is_empty() {
                let all = routers.drain(..).collect::<Vec<_>>();
                let count = all.len();
                let api_router = all.into_iter()
                    .reduce(|acc, r| acc.merge(r))
                    .unwrap();

                let mut api = OpenApi::default();
                let api_routes = api_router.finish_api(&mut api);
                main_router = main_router.merge(api_routes);

                // 存储 OpenAPI spec，供 setup_doc_routes 使用
                match serde_json::to_value(&api) {
                    Ok(spec) => { let _ = OPENAPI_SPEC.set(spec); }
                    Err(e) => error!("序列化 OpenAPI spec 失败: {}", e),
                }

                info!("已合并 {} 个路由器", count);
            } else {
                debug!("未注册任何路由器");
            }
        } else {
            error!("无法获取路由器注册表的锁");
        }

        main_router
    }

    /// 注册文档路由（在中间件之外）
    ///
    /// 包括 `/docs`（Redoc）和 `/api-docs/openapi.json`
    #[cfg(feature = "api-doc")]
    fn setup_doc_routes(router: axum::Router) -> axum::Router {
        use aide::redoc::Redoc;

        let spec = match OPENAPI_SPEC.get() {
            Some(s) => s.clone(),
            None => return router,
        };

        let router = router.route(
            "/api-docs/openapi.json",
            get(move || async { axum::response::Json(spec) }),
        );

        let redoc_html: &'static str = Box::leak(
            Redoc::new("/api-docs/openapi.json")
                .with_title("API Documentation")
                .html()
                .into_boxed_str()
        );
        router.route("/docs", get(move || async move { axum::response::Html(redoc_html) }))
    }

    #[cfg(not(feature = "api-doc"))]
    fn setup_doc_routes(router: axum::Router) -> axum::Router { router }

    /// 合并所有已注册的路由器
    #[cfg(not(feature = "api-doc"))]
    fn merge_routers() -> axum::Router {
        let mut main_router = axum::Router::new()
            .route("/health", get(health_check));

        if let Ok(mut routers) = ROUTER_REGISTRY.lock() {
            let all = routers.drain(..).collect::<Vec<_>>();
            let count = all.len();
            for router in all {
                main_router = main_router.merge(router);
            }
            info!("已合并 {} 个路由器", count);
        } else {
            error!("无法获取路由器注册表的锁");
        }

        main_router
    }

    /// 应用所有中间件层
    pub fn layer_with_router(router: axum::Router) -> axum::Router {
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
        if let Ok(mut routers) = ROUTER_REGISTRY.lock() {
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

    /// 获取已注册的路由器数量
    pub fn router_count() -> usize {
        ROUTER_REGISTRY.lock().map(|r| r.len()).unwrap_or(0)
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
async fn health_check() -> ApiR<FormattedDateTime> {
    ApiR::success(FormattedDateTime::now())
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
async fn start_server(config: Arc<WebConfig>, router: axum::Router, token: CancellationToken) -> RIE<()> {
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
