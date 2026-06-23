mod config;
mod error;
mod model;
mod mqtt;
mod protocol;
mod server;
mod util;

use crate::config::Config;
use crate::error::AppResult;
use crate::mqtt::MqttClient;
use crate::server::Server;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> AppResult<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("启动dfbja7_transformer...");

    // 加载配置
    let config = match Config::from_env() {
        Ok(config) => {
            info!("配置加载成功");
            config
        }
        Err(e) => {
            error!("配置加载失败: {}", e);
            return Err(e.into());
        }
    };
    info!("配置: {:?}", config);
    // 创建MQTT客户端
    let mqtt_client = match MqttClient::new(&config).await {
        Ok(client) => {
            info!("MQTT客户端创建成功");
            client
        }
        Err(e) => {
            error!("MQTT客户端创建失败: {}", e);
            return Err(e);
        }
    };

    // 创建并启动服务器
    let server = Server::new(config, mqtt_client);
    info!("服务器启动中...");

    if let Err(e) = server.run().await {
        error!("服务器运行错误: {}", e);
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        // 测试配置加载
        // 注意：这个测试需要设置环境变量
        // let config = Config::from_env();
        // assert!(config.is_ok());
    }
}