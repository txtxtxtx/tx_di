use std::any::{type_name, Any};
use std::sync::{Arc, LazyLock, RwLock};
use axum::{Router, routing::get};
use axum::extract::State;
use axum::http::Request;
use axum::response::IntoResponse;
use tokio::net::TcpListener;
use tower::util::ServiceFn;
use tracing::{info, error, debug};
use tx_di_core::{tx_comp, ApiR, ApiRes, App, BoxFuture, BuildContext, CompInit, FormattedDateTime, RIE};
use tx_di_log::LogPlugins;
use crate::{WebConfig, R};
use crate::bound::{AppStatus, DiComp};

/// 全局路由器注册表
///
/// 用于在应用启动前收集所有模块注册的路由器
static ROUTER_REGISTRY: LazyLock<Arc<RwLock<Vec<Router>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// 全局中间件注册表
///
/// 用于在应用启动前收集所有模块注册的中间件层（Layer）
/// 使用 Box<dyn Layer<...>> 进行类型擦除，支持存储不同类型的中间件
type ArcLayer = Box<dyn tower::Layer<Router, Service = Router> + Send + Sync>;

/// 带排序的中间件层
type SortLayer = (i32, ArcLayer);
/// 中间件层注册表
static LAYER_REGISTRY: LazyLock<Arc<RwLock<Vec<SortLayer>>>> =
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
    #[tx_cst(skip)]
    pub router: Router,
    #[tx_cst(skip)]
    pub layers: Vec<ArcLayer>,
}

impl CompInit for WebPlugin {
    fn inner_init(&mut self, _ctx: &mut BuildContext) -> RIE<()> {
        info!("Web 服务器插件初始化中...");
        self.router = WebPlugin::merge_routers()
            .route("/health", get(health_check))
            .route("/di", get(hello_di));

        Ok(())
    }
    fn async_init(ctx: Arc<App>) -> BoxFuture<'static, RIE<()>> {
        let config = ctx.inject::<WebConfig>();
        let web = ctx.inject::<WebPlugin>();
        let app_status = AppStatus { app: ctx.clone() };
        let router = web.router.clone()
            // 吧 app 注入到 extensions
            .layer(tower::ServiceBuilder::new()
            .map_request(move |mut req: Request<_>| {
                req.extensions_mut().insert(app_status.clone());
                req
            })
            .into_inner());
        Box::pin(async move {
            start_server(config, router).await?;
            Ok(())
        })
    }

    /// 插件初始化排序，最后初始化
    fn init_sort() -> i32 {
        i32::MAX
    }
}


impl WebPlugin {
    /// 添加路由器
    ///
    /// 该方法用于添加一个新的路由器，该路由器将合并到 Web 插件的路由器中。
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

    /// 添加中间件层（Layer）
    ///
    /// 该方法用于添加一个中间件层，该中间件将在服务器启动时应用到所有路由上。
    /// 中间件会按照注册的顺序依次应用。
    /// ```txt
    /// 请求流程（进入）:
    /// ┌─────────────────────────────────────┐
    /// │  Middleware A (sort=1) - 先执行      │  ← 最先处理请求
    /// │  ┌───────────────────────────────┐  │
    /// │  │  Middleware B (sort=2)        │  │
    /// │  │  ┌─────────────────────────┐  │  │
    /// │  │  │  Middleware C (sort=3)  │  │  │
    /// │  │  │  ┌───────────────────┐  │  │  │
    /// │  │  │  │   Handler         │  │  │  │  ← 业务逻辑
    /// │  │  │  └───────────────────┘  │  │  │
    /// │  │  └─────────────────────────┘  │  │
    /// │  └───────────────────────────────┘  │
    /// └─────────────────────────────────────┘
    ///
    /// 响应流程（返回）:
    /// ┌─────────────────────────────────────┐
    /// │  Middleware A (sort=1) - 后执行      │  ← 最后处理响应
    /// │  ┌───────────────────────────────┐  │
    /// │  │  Middleware B (sort=2)        │  │
    /// │  │  ┌─────────────────────────┐  │  │
    /// │  │  │  Middleware C (sort=3)  │  │  │
    /// │  │  │  ┌───────────────────┐  │  │  │
    /// │  │  │  │   Handler         │  │  │  │
    /// │  │  │  └───────────────────┘  │  │  │
    /// │  │  └─────────────────────────┘  │  │
    /// │  └───────────────────────────────┘  │
    /// └─────────────────────────────────────┘
    ///
    /// # Arguments
    ///
    /// * `layer` - 要添加的中间件层，需要实现 tower::Layer trait
    /// * `sort` - 中间件的排序序号，越小越优先级高
    /// # Example
    ///
    /// ```ignore
    /// use tower_http::cors::CorsLayer;
    /// WebPlugin::add_layer(CorsLayer::permissive());
    /// ```
    pub fn add_layer<L>(layer: L,sort: i32)
    where
        L: tower::Layer<Router, Service = Router> + Send + Sync + 'static,
    {
        if let Ok(mut layers) = LAYER_REGISTRY.write() {
            layers.push((sort,Box::new(layer)));
            info!("中间件层已注册到全局注册表: sort={}, type={}", sort, std::any::type_name::<L>());
        } else {
            error!("无法获取中间件注册表的写锁");
        }
    }

    /// 合并所有已注册的路由器并应用中间件
    ///
    /// 将全局注册表中的所有路由器合并为一个主路由器，并应用所有注册的中间件层。
    /// 中间件会按照优先级排序后依次应用（sort 值越小，优先级越高，越先执行）。
    ///
    /// # 执行顺序说明
    ///
    /// 中间件遵循"洋葱模型"：
    /// - **请求阶段**：优先级高的中间件先执行（外层 → 内层）
    /// - **响应阶段**：优先级高的中间件后执行（内层 → 外层）
    ///
    /// 例如：A(sort=1) → B(sort=2) → C(sort=3)
    /// - 请求: A → B → C → Handler
    /// - 响应: A ← B ← C ← Handler
    ///
    /// # 返回值
    ///
    /// 返回应用了所有中间件后的主路由器，如果没有任何注册的路由器，则返回空路由器
    fn merge_routers() -> Router {
        let mut main_router = Router::new();
        
        // 合并所有路由器
        if let Ok(routers) = ROUTER_REGISTRY.read() {
            for router in routers.iter() {
                main_router = main_router.merge(router.clone());
            }
            info!("已合并 {} 个路由器", routers.len());
        } else {
            error!("无法获取路由器注册表的读锁");
        }
        
        // 应用所有中间件层（按优先级排序）sort 大的在外层，sort 小的在内层
        if let Ok(mut layers) = LAYER_REGISTRY.write() {
            layers.sort_by_key(|(sort, _)| *sort);

            for (_, layer) in layers.iter() {
                main_router = layer.layer(main_router);
            }
            let layer_names: Vec<String> = layers.iter()
                .map(|(sort, layer)| format!("{}(sort={})", std::any::type_name_of_val(layer), sort))
                .collect();
            debug!("已应用 {} 个中间件层（已按优先级排序）:[{}]", layers.len(),layer_names.join(", "));
        } else {
            error!("无法获取中间件注册表的读锁");
        }
        
        main_router
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

/// 健康检查端点
///
/// 返回简单的 OK 响应，用于负载均衡器或监控系统的健康检查
async fn health_check() -> R<FormattedDateTime> {
    ApiR::success(FormattedDateTime::now()).into()
}

async fn hello_di(log_plugins: DiComp<LogPlugins>) -> R<String> {
    R::from(ApiR::success(log_plugins.config.level.as_str().into()))
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
async fn start_server(config: Arc<WebConfig>, router: Router) -> RIE<()> {
    let addr = config.socket_addr()?;

    info!("CORS 启用状态: {}", config.enable_cors);
    info!("最大请求体大小: {} bytes", config.max_body_size);
    info!("静态文件夹路径:{}",config.static_dir);
    info!("Web 服务器正在监听: {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("Web 服务器运行失败: {}", e))?;

    Ok(())
}
