use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;
use tx_di_core::{tx_comp, BuildContext, CompInit, RIE};
use crate::layers::{add_layer, add_layer_by_name};
use crate::layers::api_log::ApiLogLayer;

/// Web 服务器配置结构体
///
/// 用于配置 axum web 服务器的基本参数，通过 TOML 配置文件自动反序列化。
///
/// # 配置文件示例
///
/// ## IPv4
/// ```toml
/// [web_config]
/// host = "127.0.0.1"
/// port = 8080
/// ```
///
/// ## IPv6
/// ```toml
/// [web_config]
/// host = "::1"           # IPv6 localhost
/// # host = "::"          # 监听所有 IPv6 接口
/// port = 8080
/// ```
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf,init)]
pub struct WebConfig {
    /// 服务器监听地址
    ///
    /// 支持 IPv4 和 IPv6 地址：
    /// - IPv4: `"127.0.0.1"`, `"0.0.0.0"`
    /// - IPv6: `"::1"`, `"::"`
    ///
    /// 默认为 `"127.0.0.1"`。
    ///
    /// 该字段在 TOML 配置文件中对应 `web_config.host`。
    #[serde(default = "default_host")]
    pub host: String,

    /// 服务器监听端口
    ///
    /// 默认为 `8080`。
    ///
    /// 该字段在 TOML 配置文件中对应 `web_config.port`。
    #[serde(default = "default_port")]
    pub port: u16,

    /// 是否启用 CORS（跨域资源共享）
    ///
    /// 默认为 `false`。
    ///
    /// 该字段在 TOML 配置文件中对应 `web_config.enable_cors`。
    #[serde(default)]
    pub enable_cors: bool,

    /// 最大请求体大小（字节）
    ///
    /// 默认为 `10485760` (10MB)。
    ///
    /// 该字段在 TOML 配置文件中对应 `web_config.max_body_size`。
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    /// 静态文件目录
    #[serde(default = "default_static_dir")]
    pub static_dir: String,
    /// 中间件列表
    pub layers: Option<Vec<(i32,String)>>
}

impl CompInit for WebConfig {
    fn inner_init(&mut self, ctx: &mut BuildContext) -> RIE<()> {
        // 使用 clone() 复制 layers 的值，避免借用问题
        if let Some(layers) = &self.layers.clone() {
            for (priority, layer) in layers {
                add_layer_by_name(layer, *priority);
            }
        }
        Ok(())
    }
    fn init_sort() -> i32 {
        i32::MAX
    }
}





impl WebConfig {
    /// 获取完整的监听地址
    ///
    /// 对于 IPv6 地址，会自动添加方括号，格式为 `[host]:port`
    /// 对于 IPv4 地址，格式为 `host:port`
    pub fn address(&self) -> String {
        // 检测是否为 IPv6 地址
        if self.host.contains(':') && !self.host.starts_with('[') {
            // IPv6 地址需要方括号
            format!("[{}]:{}", self.host, self.port)
        } else {
            // IPv4 地址或已包含方括号的地址
            format!("{}:{}", self.host, self.port)
        }
    }

    /// 获取 SocketAddr
    ///
    /// 将配置转换为 SocketAddr，用于绑定服务器
    ///
    /// # Errors
    ///
    /// 如果地址格式不正确，将返回错误
    pub fn socket_addr(&self) -> RIE<SocketAddr>{
        let addr = self.address()
            .parse()
            .map_err(|e| anyhow::anyhow!("无效的地址格式 '{}': {}", self.address(), e))?;
        Ok(addr)
    }
    
    pub fn static_dir(&self) -> PathBuf {
        PathBuf::from(self.static_dir.clone())
    }
}

/// 提供默认的监听地址
fn default_host() -> String {
    "127.0.0.1".to_string()
}

/// 提供默认的端口号
fn default_port() -> u16 {
    8080
}

/// 提供默认的最大请求体大小（10MB）
fn default_max_body_size() -> usize {
    10 * 1024 * 1024
}

fn default_static_dir() -> String {
    "./static".to_string()
}