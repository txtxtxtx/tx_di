//! # 测试客户端
//!
//! 定时向 TCP 服务器推送测试数据
//!
//! ## 使用方法
//!
//! ```bash
//! cargo run --example test_client
//! ```

use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::sleep;

/// 测试数据列表（标签, 十六进制数据）
const TEST_DATA_LIST: &[(&str, &str)] = &[
    // ("文档示例信息","5528070D2D91061D000000190005004400D0000000000000000000000000000000000800000034E1"),
    ("气体一般情况", "5528070E5EBE921D000000000000000100CC000000000000000000000000000000000000001CD32B"),
    ("气体一级报警", "5528070E5EBE921D000000000000001D00CD00000000000000000000000000000000040000189CE4"),
    ("气体二级报警", "5528070E5EBE921D000000000000002F00CD00000000000000000000000000000000080000165882"),
    ("气体故障", "5528070E5EBE921D00008000FFFF000100CC000000000000000000000000000000000000001C5E75"),
    ("气体屏蔽", "5528070E5EBE921D000080000000000100CC000000000000000000000000000000000000001CBF4C"),
    ("设备报警", "5528070E5EBE921D000000000000000000CE0000000000000000000000000000000000010019CCB5"),
];


/// 服务器地址
const SERVER_ADDR: &str = "192.168.0.90:10080";

/// 推送间隔（秒）
const INTERVAL_SECS: u64 = 5;

/// 十六进制字符串转字节数组
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.replace(" ", "");
    let mut bytes = Vec::new();
    let chars: Vec<char> = hex.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let high = chars[i].to_digit(16).unwrap_or(0) as u8;
        let low = if i + 1 < chars.len() {
            chars[i + 1].to_digit(16).unwrap_or(0) as u8
        } else {
            0
        };
        bytes.push((high << 4) | low);
        i += 2;
    }

    bytes
}

/// 发送单条测试数据
async fn send_test_data(label: &str, hex: &str) -> anyhow::Result<()> {
    let data = hex_to_bytes(hex);

    println!("连接到服务器: {}", SERVER_ADDR);
    let mut stream = TcpStream::connect(SERVER_ADDR).await?;

    println!("[{}] 发送数据: {} 字节", label, data.len());
    println!("十六进制: {}", hex);

    stream.write_all(&data).await?;
    stream.flush().await?;

    println!("[{}] 数据发送成功！", label);

    // 等待一会儿再关闭连接
    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 测试客户端 ===");
    println!("服务器地址: {}", SERVER_ADDR);
    println!("推送间隔: {} 秒", INTERVAL_SECS);
    println!("测试数据数量: {}", TEST_DATA_LIST.len());
    println!("");

    for (i, (label, hex)) in TEST_DATA_LIST.iter().enumerate() {
        let count = i + 1;
        println!("\n--- 第 {}/{} 次推送: {} ---", count, TEST_DATA_LIST.len(), label);

        match send_test_data(label, hex).await {
            Ok(_) => {
                println!("[{}] 推送成功", label);
            }
            Err(e) => {
                eprintln!("[{}] 推送失败: {}", label, e);
            }
        }

        if count < TEST_DATA_LIST.len() {
            println!("等待 {} 秒后继续...", INTERVAL_SECS);
            sleep(Duration::from_secs(INTERVAL_SECS)).await;
        }
    }

    println!("\n=== 全部 {} 条测试数据推送完成 ===", TEST_DATA_LIST.len());
    Ok(())
}
