//! Bootstrap — 本地配置加载（bootstrap 层）
//!
//! 应用启动最早阶段：读取本地 TOML 中的 `[registry_config]`（Nacos 地址等），
//! 供 `NacosClient` 连接配置中心。本地业务配置作为「配置中心缺失/不可达」时的兜底。

use tx_di_core::RIE;
use tx_error::AppError;

use crate::config::RegistryConfig;

/// 读取本地 TOML 配置（文件不存在 → 空 Table，允许无配置运行）
pub fn load_local_toml(config_path: &str) -> RIE<toml::Value> {
    // 先加载 .env，再替换配置中的 `${VAR}` 占位符
    tx_di_core::ensure_dotenv();
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            let content = tx_di_core::interpolate_env(&content).map_err(|e| {
                AppError::from(format!("配置文件环境变量替换失败: {config_path:?}\n{e}"))
            })?;
            toml::from_str(&content).map_err(|e| {
                AppError::from(format!(
                    "配置文件解析失败: {:?}\n错误: {}\n请检查 TOML 语法是否正确。",
                    config_path, e
                ))
            })
        }
        Err(_) => {
            tracing::warn!("配置文件不存在: {:?}，将使用默认配置", config_path);
            Ok(toml::Value::Table(toml::map::Map::new()))
        }
    }
}

/// 读取 bootstrap 配置（仅 `[registry_config]` 节，控制 Nacos 接入）
pub fn load_bootstrap(config_path: &str) -> RIE<RegistryConfig> {
    let local = load_local_toml(config_path)?;
    let cfg: RegistryConfig = local
        .get("registry_config")
        .and_then(|v| v.clone().try_into().ok())
        .unwrap_or_default();
    tracing::info!(
        enabled = cfg.enabled,
        service_name = %cfg.service_name,
        nacos_addr = %cfg.nacos_addr,
        "bootstrap 配置已加载"
    );
    Ok(cfg)
}
