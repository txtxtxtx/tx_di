//! 配置发布工具：向 Nacos 发布/删除指定 dataId 的配置（用于触发配置变更重启验证）
//!
//! 用法：
//!   NACOS_USER=nacos NACOS_PASS=xxx \
//!   cargo run -p tx_di_nacos --example publish -- <dataId> <content-or-remove>
//!   示例：
//!     cargo run -p tx_di_nacos --example publish -- tx_admin_app_loop.toml "[demo]\nmessage=\"hello-v2\""
//!     cargo run -p tx_di_nacos --example publish -- tx_admin_app_loop.toml remove

use tx_di_nacos::NacosClient;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: publish <dataId> <content|remove|@file>");
        std::process::exit(1);
    }
    let data_id = args[1].as_str();
    let action = args[2].as_str();

    let cfg = tx_di_nacos::RegistryConfig {
        enabled: true,
        nacos_addr: std::env::var("NACOS_ADDR").unwrap_or_else(|_| "http://192.168.0.91:8848".into()),
        namespace: std::env::var("NACOS_NS").unwrap_or_else(|_| "yc_dev".into()),
        group: std::env::var("NACOS_GROUP").unwrap_or_else(|_| "DEFAULT_GROUP".into()),
        service_name: "tx_admin_publish".into(),
        auto_register: false,
        heartbeat_secs: 5,
        config_data_id: None,
        username: std::env::var("NACOS_USER").ok().filter(|s| !s.is_empty()),
        password: std::env::var("NACOS_PASS").ok().filter(|s| !s.is_empty()),
    };

    let client = match NacosClient::connect(&cfg).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 连接失败: {e}");
            std::process::exit(1);
        }
    };

    let group = &cfg.group;
    if action == "remove" {
        match client.config.remove_config(data_id, group).await {
            Ok(()) => println!("✅ 已删除配置 {data_id}"),
            Err(e) => {
                eprintln!("❌ 删除失败: {e}");
                std::process::exit(1);
            }
        }
    } else {
        // 支持 @file 读取内容（避免 shell 引号问题）
        let content = if let Some(path) = action.strip_prefix('@') {
            std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("❌ 读取文件失败: {e}");
                std::process::exit(1);
            })
        } else {
            // 将 \n 转义还原为换行
            action.replace("\\n", "\n")
        };
        match client.config.publish_config(data_id, group, &content).await {
            Ok(()) => println!("✅ 已发布配置 {data_id}:\n{content}"),
            Err(e) => {
                eprintln!("❌ 发布失败: {e}");
                std::process::exit(1);
            }
        }
    }
}
