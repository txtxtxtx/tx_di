//! Nacos 冒烟测试：验证 SDK 连接 + 配置中心 + 服务注册完整功能
//!
//! 用法：
//!   cargo run -p tx_di_nacos --example smoke
//!
//! 通过环境变量覆盖默认连接参数（可选）：
//!   NACOS_ADDR   默认 http://192.168.0.91:8848
//!   NACOS_NS     默认 yc_dev
//!   NACOS_GROUP  默认 DEFAULT_GROUP
//!   NACOS_USER   默认 ""（不鉴权）
//!   NACOS_PASS   默认 ""（不鉴权）

use std::time::Duration;

use tx_di_nacos::{NacosClient, Protocol, RegistryConfig, ServiceEndpoint};

fn registry_config() -> RegistryConfig {
    RegistryConfig {
        enabled: true,
        nacos_addr: std::env::var("NACOS_ADDR").unwrap_or_else(|_| "http://192.168.0.91:8848".into()),
        namespace: std::env::var("NACOS_NS").unwrap_or_else(|_| "yc_dev".into()),
        group: std::env::var("NACOS_GROUP").unwrap_or_else(|_| "DEFAULT_GROUP".into()),
        service_name: "tx_admin_smoke".into(),
        auto_register: true,
        heartbeat_secs: 5,
        config_data_id: Some("tx_admin_smoke.toml".into()),
        username: std::env::var("NACOS_USER").ok().filter(|s| !s.is_empty()),
        password: std::env::var("NACOS_PASS").ok().filter(|s| !s.is_empty()),
    }
}

#[tokio::main]
async fn main() {
    println!("═══ Nacos 冒烟测试开始 ═══");
    let cfg = registry_config();
    println!("连接: {} ns={} group={}", cfg.nacos_addr, cfg.namespace, cfg.group);

    // 1. 连接
    let client = match NacosClient::connect(&cfg).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 连接失败: {e}");
            std::process::exit(1);
        }
    };
    println!("✅ 连接成功");

    // 2. 配置中心：发布 → 拉取 → 删除
    let data_id = "tx_admin_smoke.toml";
    let group = &cfg.group;
    let content = r#"
[smoke]
enabled = true
message = "hello from nacos 3.2"
"#;
    println!("[配置] 发布 {data_id} ...");
    match client.config.publish_config(data_id, group, content).await {
        Ok(()) => println!("✅ 配置发布成功"),
        Err(e) => {
            eprintln!("❌ 配置发布失败: {e}");
            std::process::exit(1);
        }
    }

    println!("[配置] 拉取 {data_id} ...");
    match client.config.get_config(data_id, group).await {
        Ok(Some(v)) => {
            println!("✅ 配置拉取成功:\n{v}");
        }
        Ok(None) => println!("⚠️ 配置不存在"),
        Err(e) => {
            eprintln!("❌ 配置拉取失败: {e}");
            std::process::exit(1);
        }
    }

    // 3. 服务注册：注册 → 发现 → 注销
    let endpoints = vec![ServiceEndpoint {
        protocol: Protocol::Http,
        ip: "127.0.0.1".into(),
        port: 18888,
        metadata: Default::default(),
    }];
    println!("[服务] 注册 {} ...", cfg.service_name);
    let instance_id = match client.register_service(endpoints).await {
        Ok(id) => {
            println!("✅ 服务注册成功 instance_id={id}");
            id
        }
        Err(e) => {
            eprintln!("❌ 服务注册失败: {e}");
            std::process::exit(1);
        }
    };

    // 等待注册中心收敛
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("[服务] 发现 {} ...", cfg.service_name);
    match client.naming.discover(&cfg.service_name).await {
        Ok(instances) => {
            println!("✅ 服务发现成功，实例数={}", instances.len());
            for inst in instances {
                println!("   - {} healthy={} eps={:?}", inst.instance_id, inst.healthy, inst.endpoints);
            }
        }
        Err(e) => {
            eprintln!("❌ 服务发现失败: {e}");
        }
    }

    // 4. 注销
    println!("[服务] 注销 {instance_id} ...");
    match client.deregister(&instance_id).await {
        Ok(()) => println!("✅ 服务注销成功"),
        Err(e) => eprintln!("❌ 服务注销失败: {e}"),
    }

    // 5. 清理测试配置
    match client.config.remove_config(data_id, group).await {
        Ok(()) => println!("✅ 测试配置已清理"),
        Err(e) => eprintln!("⚠️ 测试配置清理失败: {e}"),
    }

    println!("═══ 冒烟测试结束 ═══");
}
