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

/// 测试数据（十六进制格式）
const TEST_DATA_HEX: &str = "5528070D2D91061D000000190005004400D0000000000000000000000000000000000800000034E1";

/// 服务器地址
const SERVER_ADDR: &str = "127.0.0.1:10080";

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

/// 发送测试数据
async fn send_test_data() -> anyhow::Result<()> {
    let data = hex_to_bytes(TEST_DATA_HEX);

    println!("连接到服务器: {}", SERVER_ADDR);
    let mut stream = TcpStream::connect(SERVER_ADDR).await?;

    println!("发送数据: {} 字节", data.len());
    println!("十六进制: {}", TEST_DATA_HEX);

    stream.write_all(&data).await?;
    stream.flush().await?;

    println!("数据发送成功！");

    // 等待一会儿再关闭连接
    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 测试客户端 ===");
    println!("服务器地址: {}", SERVER_ADDR);
    println!("推送间隔: {} 秒", INTERVAL_SECS);
    println!("测试数据: {}", TEST_DATA_HEX);
    println!("按 Ctrl+C 停止");
    println!("");

    let mut count = 0;

    loop {
        count += 1;
        println!("\n--- 第 {} 次推送 ---", count);

        match send_test_data().await {
            Ok(_) => {
                println!("推送成功");
            }
            Err(e) => {
                eprintln!("推送失败: {}", e);
            }
        }

        println!("等待 {} 秒后继续...", INTERVAL_SECS);
        sleep(Duration::from_secs(INTERVAL_SECS)).await;
    }
}
