//! sa-token 核心插件组件

use crate::config::SaTokenConf;
use std::sync::{Arc, OnceLock};
use tx_di_core::{App, Component, DepsTuple, RIE};
use tracing::info;

/// sa-token 全局状态（进程级单例）
///
/// sa-token-core 的 `StpUtil::init_manager` 内部使用 `OnceLock`（`GLOBAL_MANAGER`），
/// 进程内只能初始化一次，且**没有 reset/重新初始化 API**（重复 set 直接 panic）。
/// 而 tx-di 的 `app_loop!` 支持"配置变更 → 优雅重启（不退出进程）"，
/// 重启会重新构建组件并再次走到 `SaTokenStateBuilder::build()` → 触发 panic
/// （`StpUtil manager already initialized`）。
///
/// 因此这里将构建结果缓存到进程级静态变量：首次 build 后复用，后续重启不再 build。
///
/// # TODO（已知缺陷，配置变更无法生效）
///
/// 当前"复用已构建状态"是**临时妥协方案**，存在功能缺陷：
///
/// 1. **sa-token 配置变更不生效**：Nacos 配置中心修改 `[sa_token]`（如 `token_name`、
///    `timeout`、`is_concurrent`、`storage` 后端）后，`app_loop!` 虽优雅重启了 App，
///    但 SaTokenState 仍是旧配置构建的（`GLOBAL_SA_TOKEN_STATE` 未更新）——
///    对外表现是"配置已更新但 sa-token 行为未变"，属于**静默失效**（比 panic 更隐蔽）。
/// 2. **存储后端不可切换**：memory ↔ redis 也无法在进程内热切换。
///
/// 正确方案（择一实施）：
/// - **A（推荐）**：sa-token-core 提供 `reset`/重新初始化能力（如将 `GLOBAL_MANAGER`
///   改为 `RwLock<Option<Arc<SaTokenManager>>>`，`init_manager` 支持覆盖），
///   依赖方升级后此处改为"每次重启重建 SaTokenState 并 reset 全局 manager"。
/// - **B**：fork sa-token-core（或提交 PR），增加 `StpUtil::rebuild(manager)` 方法。
/// - **C（兜底）**：文档明确 `[sa_token]` 配置变更需**完整重启进程**生效
///   （`app_loop!` 只保证业务配置热更，sa-token 属初始化型配置例外）。
static GLOBAL_SA_TOKEN_STATE: OnceLock<sa_token_plugin_axum::SaTokenState> = OnceLock::new();

/// sa-token 插件
///
/// 封装 sa-token-rust 的初始化逻辑，包括：
/// - 构建并配置 `SaTokenState`
/// - 提供 `StpUtil` 工具类进行登录/权限操作
/// - 与 tx_di_axum 集成的 Layer
///
/// # DI 注入方式
///
/// ```rust,ignore
/// #[tx_comp(init)]
/// pub struct MyService {
///     pub sa_token: Arc<SaTokenPlugin>,
/// }
/// ```
///
/// # 使用方式
///
/// ```rust,ignore
/// // 登录
/// StpUtil::login("user_10001", &sa_token_state);
///
/// // 检查登录
/// StpUtil::is_login(&sa_token_state);
///
/// // 获取当前登录 ID
/// StpUtil::get_login_id(&sa_token_state);
///
/// // 注销
/// StpUtil::logout(&sa_token_state);
/// ```
#[derive(Component)]
#[component(app_async_init, init_sort = i32::MIN + 1)]
pub struct SaTokenPlugin {
    /// 配置引用
    pub config: Arc<SaTokenConf>,

    /// SaToken 状态实例
    ///
    /// 通过 `OnceLock` 延迟初始化，因为 `SaTokenState` 的构建需要在 `async_init` 阶段完成。
    #[tx_cst(std::sync::OnceLock::new())]
    pub state: std::sync::OnceLock<sa_token_plugin_axum::SaTokenState>,
}

impl SaTokenPlugin {
    /// 获取已初始化的 SaTokenState 引用
    ///
    /// 必须在 `async_init` 完成后调用，否则 panic。
    pub fn state(&self) -> &sa_token_plugin_axum::SaTokenState {
        self.state
            .get()
            .expect("SaTokenPlugin: state not initialized yet, async_init not completed")
    }

    /// 尝试获取 SaTokenState 引用（安全版本）
    pub fn try_state(&self) -> Option<&sa_token_plugin_axum::SaTokenState> {
        self.state.get()
    }
}

/// `#[component(app_async_init)]` 回调：构建 SaToken 状态
///
/// 异步构建：Redis 存储后端需要异步建连，因此从 `init`（同步）移到 `async_init` 阶段。
/// 依赖 SaToken 状态的组件需在 `async_init` 之后使用（`state()` 已做防御性 panic）。
async fn app_async_init(comp: Arc<SaTokenPlugin>, _app: Arc<App>) -> RIE<()> {
    info!("SaTokenPlugin 初始化");
    let config = comp.config.clone();

    // 进程级复用：首次构建，重启复用（避免 StpUtil manager already initialized panic）
    //
    // TODO(bug): 复用会导致 sa-token 配置（[sa_token]）更新后不生效——重启后仍用旧配置构建的
    // state。正确行为是"配置变更 → 重建 SaTokenState"，受限于 sa-token-core 全局 OnceLock
    // 无 reset API（详见 GLOBAL_SA_TOKEN_STATE 文档注释）。实施修复方案后删除此分支的复用逻辑。
    let state = match GLOBAL_SA_TOKEN_STATE.get() {
        Some(s) => {
            info!("复用已构建的 SaToken 状态（进程内优雅重启）");
            s.clone()
        }
        None => {
            info!("正在构建 SaToken 状态...");
            // 使用 Builder 模式构建 SaTokenState
            let builder = sa_token_plugin_axum::SaTokenStateBuilder::default();
            let state = config.apply_to_builder(builder).await?.build();
            // 首次构建成功后才写入全局（set 失败说明并发下已被其他任务构建，忽略）
            let _ = GLOBAL_SA_TOKEN_STATE.set(state.clone());
            state
        }
    };

    // 写入组件字段 OnceLock
    if comp.state.set(state).is_err() {
        tracing::warn!("SaTokenPlugin: state concurrently initialized");
    }
    info!(
            token_name = %config.token_name,
            timeout = config.timeout,
            "SaToken 初始化完成"
        );
    Ok(())
}

// ── Axum 集成辅助 ─────────────────────────────────────────────────────────────

impl SaTokenPlugin {
    /// 构建用于 Axum Router 的 SaTokenLayer
    ///
    /// ```rust,ignore
    /// let layer = sa_token_plugin.build_layer();
    /// let app = Router::new()
    ///     .route("/api/protected", get(handler))
    ///     .layer(layer);
    /// ```
    pub fn build_layer(&self) -> sa_token_plugin_axum::SaTokenLayer {
        sa_token_plugin_axum::SaTokenLayer::new(self.state().clone())
    }

    /// 构建带路径鉴权配置的 SaTokenLayer
    ///
    /// ```rust,ignore
    /// let path_auth = PathAuthConfig::new()
    ///     .add_include_pattern("/api/**")
    ///     .add_exclude_pattern("/api/public/**");
    /// let layer = sa_token_plugin.build_layer_with_path_auth(path_auth);
    /// ```
    pub fn build_layer_with_path_auth(
        &self,
        path_auth: sa_token_plugin_axum::sa_token_core::router::PathAuthConfig,
    ) -> sa_token_plugin_axum::SaTokenLayer {
        sa_token_plugin_axum::SaTokenLayer::with_path_auth(self.state().clone(), path_auth)
    }

    /// 构建登录检查 Layer（要求用户已登录）
    pub fn check_login_layer(&self) -> sa_token_plugin_axum::SaCheckLoginLayer {
        sa_token_plugin_axum::SaCheckLoginLayer::new()
    }

    /// 构建权限检查 Layer（要求特定权限）
    pub fn check_permission_layer(
        &self,
        permission: impl Into<String>,
    ) -> sa_token_plugin_axum::SaCheckPermissionLayer {
        sa_token_plugin_axum::SaCheckPermissionLayer::new(permission)
    }
}
