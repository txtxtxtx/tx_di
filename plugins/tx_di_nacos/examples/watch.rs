//! 配置变更监听测试：验证 `watch_config` 在远端配置变更时能收到通知
//!
//! 用法：
//!   NACOS_USER=nacos NACOS_PASS=xxx cargo run -p tx_di_nacos --example watch
//!
//! 流程：发布配置 A → 等待通知① → 发布配置 B → 等待通知② → 清理

use std::time::Duration;

use tx_di_nacos::NacosClient;

#[tokio::main]
async fn main() {
    println!("═══ 配置变更监听测试 ═══");
    let cfg = tx_di_nacos::RegistryConfig {
        enabled: true,
        nacos_addr: std::env::var("NACOS_ADDR").unwrap_or_else(|_| "http://192.168.0.91:8848".into()),
        namespace: std::env::var("NACOS_NS").unwrap_or_else(|_| "yc_dev".into()),
        group: std::env::var("NACOS_GROUP").unwrap_or_else(|_| "DEFAULT_GROUP".into()),
        service_name: "tx_admin_watch".into(),
        auto_register: false,
        heartbeat_secs: 5,
        config_data_id: Some("tx_admin_watch.toml".into()),
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
    println!("✅ 连接成功");

    let data_id = "tx_admin_watch.toml";
    let group = &cfg.group;
    let mut rx = client.watch_config(data_id);
    println!("✅ 已注册配置监听");

    // 发布配置 A
    let content_a = "[watch]\nversion = 1\n";
    if let Err(e) = client.config.publish_config(data_id, group, content_a).await {
        eprintln!("❌ 发布配置 A 失败: {e}");
        std::process::exit(1);
    }
    println!("✅ 已发布配置 A (version=1)");

    // 等待通知 ①
    match tokio::time::timeout(Duration::from_secs(10), rx.changed()).await {
        Ok(Ok(())) => println!("✅ 收到变更通知①，当前版本={}", *rx.borrow()),
        Ok(Err(e)) => println!("⚠️ 通知① channel 关闭: {e}"),
        Err(_) => println!("⚠️ 通知① 超时（10s 未收到）"),
    }

    // 稍等，然后发布配置 B（version=2）
    tokio::time::sleep(Duration::from_secs(2)).await;
    let content_b = "[watch]\nversion = 2\n";
    if let Err(e) = client.config.publish_config(data_id, group, content_b).await {
        eprintln!("❌ 发布配置 B 失败: {e}");
        std::process::exit(1);
    }
    println!("✅ 已发布配置 B (version=2)");

    // 等待通知 ②
    match tokio::time::timeout(Duration::from_secs(10), rx.changed()).await {
        Ok(Ok(())) => println!("✅ 收到变更通知②，当前版本={}", *rx.borrow()),
        Ok(Err(e)) => println!("⚠️ 通知② channel 关闭: {e}"),
        Err(_) => println!("⚠️ 通知② 超时（10s 未收到）"),
    }

    // 清理
    let _ = client.config.remove_config(data_id, group).await;
    println!("═══ 测试结束 ═══");
}
