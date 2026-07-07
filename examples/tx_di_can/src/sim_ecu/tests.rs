//! ECU 仿真节点集成测试
//!
//! 在 SimBus 上启动仿真节点，用 UdsClient / FlashEngine 作为 tester 发起请求，
//! 验证"发请求 → 收到正确 ECU 响应"的 round-trip，覆盖无设备联调的核心路径。

use super::*;
use crate::adapter::SimBusAdapter;
use crate::db::DescDb;
use crate::flash::{FlashConfig, FlashEngine};
use crate::isotp::IsoTpConfig;
use crate::uds::UdsClient;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

/// 在 SimBus 上启动仿真并返回 tester 客户端与停止标志
async fn setup(cfg: SimEcuConfig) -> (UdsClient, Arc<AtomicBool>) {
    let adapter = Arc::new(SimBusAdapter::new("sim_test", 1024));
    adapter.open().await.unwrap();
    let db = Arc::new(DescDb::builtin());
    let running = Arc::new(AtomicBool::new(true));
    spawn_sim_ecu(adapter.clone(), cfg.clone(), db, running.clone());
    // 等待仿真任务完成订阅，避免请求早于订阅发出
    tokio::time::sleep(Duration::from_millis(50)).await;

    let tester = UdsClient::new(
        adapter,
        IsoTpConfig {
            tx_id: cfg.req_id,
            rx_id: cfg.resp_id,
            is_fd: cfg.is_fd,
            ..Default::default()
        },
        150,
        5000,
    );
    (tester, running)
}

#[tokio::test]
async fn sim_read_did_vin() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    let data = ecu.read_data(0xF190).await.expect("read VIN 应成功");
    assert_eq!(data, b"1HGCM82633A004352");
}

#[tokio::test]
async fn sim_read_write_did() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    ecu.write_data(0x010B, &[0x64]).await.expect("write 应成功");
    let data = ecu.read_data(0x010B).await.expect("read 应成功");
    assert_eq!(data, vec![0x64]);
}

#[tokio::test]
async fn sim_read_unsupported_did_nrc() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    let err = ecu.read_data(0xDEAD).await.unwrap_err();
    assert!(format!("{err}").contains("0x31") || format!("{err}").contains("RequestOutOfRange"));
}

#[tokio::test]
async fn sim_session_and_tester_present() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    ecu.session_control(crate::uds::SessionType::Extended)
        .await
        .expect("session 应成功");
    ecu.tester_present().await.expect("tester present 应成功");
}

#[tokio::test]
async fn sim_security_access() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    // key_fn 使用与 ECU 相同的算法（level=0x01 -> Xor）
    ecu.security_access(0x01, |seed| seedkey::compute_key(seed, 0x01))
        .await
        .expect("security access 应成功");
}

#[tokio::test]
async fn sim_read_dtc() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    let dtcs = ecu.read_dtc(0xFF).await.expect("read dtc 应成功");
    assert!(!dtcs.is_empty(), "应返回至少一个内置 DTC");
}

#[tokio::test]
async fn sim_fd_read_did() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: true,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    let data = ecu.read_data(0xF190).await.expect("FD read VIN 应成功");
    assert_eq!(data, b"1HGCM82633A004352");
}

#[tokio::test]
async fn sim_flash_7_steps() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;

    // 准备固件（带若干非 0xFF 数据，避免被跳过）
    let firmware: Vec<u8> = (0..2000u32).map(|i| (i as u8).wrapping_add(1)).collect();
    let path = std::env::temp_dir().join(format!("tx_di_can_sim_flash_{}.bin", std::process::id()));
    std::fs::write(&path, &firmware).unwrap();

    let flash_cfg = FlashConfig {
        target_id: 0x7E0,
        security_level: 0x01,
        session_type: crate::uds::SessionType::Programming,
        memory_address: 0x0800_0000,
        ..Default::default()
    };
    let engine = FlashEngine::new(Arc::new(ecu), flash_cfg);
    let result = engine
        .flash(&path, |seed| seedkey::compute_key(seed, 0x01))
        .await;
    let _ = std::fs::remove_file(&path);

    result.expect("刷写 7 步应成功完成");
}

#[tokio::test]
async fn sim_flash_requires_security() {
    // 关闭安全校验要求，但要求编程会话；验证顺序错误会被拒绝
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: false,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;

    // 直接请求下载（未进入编程会话）应被拒绝（ConditionsNotCorrect 0x22）
    let err = ecu
        .request_download(0x0800_0000, 256, 0x00, 0x00, 4, 4)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("0x22") || format!("{err}").contains("ConditionsNotCorrect"));
}

#[tokio::test]
async fn sim_read_multi_did() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    let resp = ecu
        .read_data_multi(&[0xF190, 0xF195, 0x010B])
        .await
        .expect("多 DID 读应成功");
    let map = UdsClient::parse_data_by_id(
        &resp,
        &[(0xF190, 17), (0xF195, 4), (0x010B, 1)],
    )
    .expect("解析应成功");
    assert_eq!(map[&0xF190], b"1HGCM82633A004352");
    assert_eq!(map[&0xF195], vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(map[&0x010B], vec![0x38]);
}

#[tokio::test]
async fn sim_read_dtc_subfn() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    // 0x01 返回数量（可能无 DTC 记录），仅校验不报错
    let _ = ecu.read_dtc_subfn(0x01, 0xFF).await.expect("sub-fn 0x01 应成功");
    // 0x0A 返回受支持 DTC 列表（含记录）
    let dtcs2 = ecu.read_dtc_subfn(0x0A, 0xFF).await.expect("sub-fn 0x0A 应成功");
    assert!(!dtcs2.is_empty());
}

#[tokio::test]
async fn sim_flash_from_s19() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;

    // 构造 S19 固件（少量数据，避免被跳过）
    let firmware: Vec<u8> = (0..500u32).map(|i| (i as u8).wrapping_add(1)).collect();
    let mut s19 = String::new();
    for (i, chunk) in firmware.chunks(16).enumerate() {
        let addr = 0x0800_0000u32 + (i * 16) as u32;
        // S3: count = 4(addr) + data + 1(checksum)
        let count = chunk.len() as u8 + 5;
        let mut rec = vec![
            count,
            (addr >> 24) as u8,
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            addr as u8,
        ];
        let mut sum = rec.iter().map(|&b| b as u32).sum::<u32>();
        for &b in chunk {
            rec.push(b);
            sum += b as u32;
        }
        let checksum = ((-(sum as i32)) & 0xFF) as u8;
        rec.push(checksum);
        let hex: String = rec.iter().map(|b| format!("{b:02X}")).collect();
        s19.push_str(&format!("S3{hex}\n"));
    }
    s19.push_str("S70508000000FA\n");

    let path = std::env::temp_dir().join(format!("tx_di_can_sim_flash_{}.s19", std::process::id()));
    std::fs::write(&path, &s19).unwrap();

    let flash_cfg = FlashConfig {
        target_id: 0x7E0,
        security_level: 0x01,
        session_type: crate::uds::SessionType::Programming,
        memory_address: 0x0800_0000,
        ..Default::default()
    };
    let engine = FlashEngine::new(Arc::new(ecu), flash_cfg);
    let result = engine
        .flash(&path, |seed| seedkey::compute_key(seed, 0x01))
        .await;
    let _ = std::fs::remove_file(&path);
    result.expect("S19 固件刷写应成功");
}

#[tokio::test]
async fn sim_routine_erase() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;
    // 显式擦除例程 0xFF00 需先进入编程会话
    ecu.session_control(crate::uds::SessionType::Programming)
        .await
        .expect("进入编程会话应成功");
    let resp = ecu
        .routine_control(0x01, 0xFF00, &[])
        .await
        .expect("擦除例程应成功");
    // 响应去掉 SID(0x71) 后为：sub_fn(0x01) rid(0xFF00) status(0x00)
    assert_eq!(resp, vec![0x01, 0xFF, 0x00, 0x00]);
}

#[tokio::test]
async fn sim_request_download_rejects_compression() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: false,
        require_programming_session: false,
    };
    let (ecu, _running) = setup(cfg).await;
    // 仅支持 0x00（无压缩/无加密）；协商非 0 方法应被拒绝
    let err = ecu
        .request_download(0x0800_0000, 256, 0x01, 0x00, 4, 4)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("0x31") || format!("{err}").contains("RequestOutOfRange"));
}

#[tokio::test]
async fn sim_flash_with_erase() {
    let cfg = SimEcuConfig {
        req_id: 0x7E0,
        resp_id: 0x7E8,
        is_fd: false,
        require_security_for_flash: true,
        require_programming_session: true,
    };
    let (ecu, _running) = setup(cfg).await;

    let firmware: Vec<u8> = (0..1500u32).map(|i| (i as u8).wrapping_add(1)).collect();
    let path = std::env::temp_dir().join(format!("tx_di_can_sim_erase_{}.bin", std::process::id()));
    std::fs::write(&path, &firmware).unwrap();

    let flash_cfg = FlashConfig {
        target_id: 0x7E0,
        security_level: 0x01,
        session_type: crate::uds::SessionType::Programming,
        memory_address: 0x0800_0000,
        erase_before_download: true,
        erase_routine_id: 0xFF,
        ..Default::default()
    };
    let engine = FlashEngine::new(Arc::new(ecu), flash_cfg);
    let result = engine
        .flash(&path, |seed| seedkey::compute_key(seed, 0x01))
        .await;
    let _ = std::fs::remove_file(&path);
    result.expect("带显式擦除的刷写应成功");
}
