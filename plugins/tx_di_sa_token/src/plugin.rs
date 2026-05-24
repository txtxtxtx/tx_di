//! sa-token 核心插件组件

use crate::config::SaTokenConf;
use std::sync::Arc;
use tx_di_core::{tx_comp, CompInit, InnerContext, RIE};
use tracing::info;

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
#[tx_comp(init)]
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

impl CompInit for SaTokenPlugin {
    fn inner_init(&mut self, _ctx: &InnerContext) -> RIE<()> {
        info!("SaTokenPlugin 初始化");
        let config = self.config.clone();
        info!("正在构建 SaToken 状态...");
        // 使用 Builder 模式构建 SaTokenState
        let builder = sa_token_plugin_axum::SaTokenStateBuilder::default();
        let state = config.apply_to_builder(builder).build();
        // 写入 OnceLock
        if self.state.set(state).is_err() {
            tracing::warn!("SaTokenPlugin: state concurrently initialized");
        }
        info!(
                token_name = %config.token_name,
                timeout = config.timeout,
                "SaToken 初始化完成"
            );
        Ok(())
    }

    fn init_sort() -> i32 {
        // 在 SaTokenConf(90) 之后、业务组件之前
        i32::MIN + 1
    }
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
