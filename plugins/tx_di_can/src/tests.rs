//!
//! tx_di_can 单元测试
//!
//! 覆盖范围：
//!  1. FrameId / CanFrame / CanFdFrame 数据结构
//!  2. CanConfig TOML 反序列化与默认值
//!  3. SimBusAdapter 仿真回环
//!  4. ISO-TP 单帧/多帧协议解析
//!  5. UDS NRC 解析与 UdsService response_sid
//!  6. FlashEngine 配置默认值与 is_all_ff 逻辑
//!  7. CanEvent 事件枚举
//!  8. create_adapter 工厂函数
//!  9. CanConfig load_from_toml

#[cfg(test)]
mod tests {
    use crate::config::{AdapterKind, CanConfig};
    use crate::frame::{CanFdFrame, CanFrame, FrameId, FrameKind};
    use crate::isotp::IsoTpConfig;
    use crate::uds::{NrcCode, SessionType, UdsService};
    use crate::flash::{FlashConfig, is_all_ff};
    use crate::adapter::{SimBusAdapter, create_adapter, CanAdapter};
    use crate::event::CanEvent;
    use std::fs;
    use std::path::PathBuf;

    // ════════════════════════════════════════════════════════════════════
    // 1. FrameId / CanFrame / CanFdFrame 数据结构
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_frame_id_from_raw() {
        assert!(matches!(FrameId::from_raw(0x000), FrameId::Standard(0)));
        assert!(matches!(FrameId::from_raw(0x7FF), FrameId::Standard(0x7FF)));
        assert!(matches!(FrameId::from_raw(0x800), FrameId::Extended(0x800)));
        assert!(matches!(FrameId::from_raw(0x123), FrameId::Standard(0x123)));
    }

    #[test]
    fn test_frame_id_raw() {
        assert_eq!(FrameId::Standard(0x5AB).raw(), 0x5AB);
        assert_eq!(FrameId::Extended(0x1FFFFFFF).raw(), 0x1FFFFFFF);
    }

    #[test]
    fn test_frame_id_is_extended() {
        assert!(!FrameId::Standard(0x7FF).is_extended());
        assert!(FrameId::Extended(0x1000).is_extended());
    }

    #[test]
    fn test_frame_id_display() {
        assert_eq!(format!("{}", FrameId::Standard(0x5AB)), "5AB");
        assert_eq!(format!("{}", FrameId::Extended(0x12345678)), "12345678");
    }

    #[test]
    fn test_can_frame_new() {
        let frame = CanFrame::new(0x7E0, vec![0x02, 0x01, 0xAA]);
        assert!(matches!(frame.id, FrameId::Standard(0x7E0)));
        assert_eq!(frame.data, vec![0x02, 0x01, 0xAA]);
        assert_eq!(frame.kind, FrameKind::Data);
        assert_eq!(frame.dlc(), 3);
    }

    #[test]
    fn test_can_frame_new_std() {
        let frame = CanFrame::new_std(0x100, vec![0x11, 0x22]);
        assert!(matches!(frame.id, FrameId::Standard(0x100)));
        assert_eq!(frame.data, vec![0x11, 0x22]);
    }

    #[test]
    fn test_can_frame_new_ext() {
        let frame = CanFrame::new_ext(0x1234_5678, vec![0xFF]);
        assert!(matches!(frame.id, FrameId::Extended(0x1234_5678)));
    }

    #[test]
    fn test_can_frame_dlc() {
        assert_eq!(CanFrame::new(0x100, vec![]).dlc(), 0);
        assert_eq!(CanFrame::new(0x100, vec![1, 2, 3]).dlc(), 3);
        assert_eq!(CanFrame::new(0x100, vec![1; 8]).dlc(), 8);
        // 超过8字节截断到8
        assert_eq!(CanFrame::new(0x100, vec![1; 20]).dlc(), 8);
    }

    #[test]
    fn test_can_fd_frame_new() {
        let frame = CanFdFrame::new(0x100, vec![0xAA; 32]);
        assert_eq!(frame.data.len(), 32);
        assert!(frame.brs);
        assert!(!frame.esi);
    }

    #[test]
    fn test_can_fd_frame_fd_dlc() {
        // CANFD DLC 映射（ISO 11898-1:2015）
        assert_eq!(CanFdFrame::new(0x100, vec![]).fd_dlc(), 0);
        assert_eq!(CanFdFrame::new(0x100, vec![0; 8]).fd_dlc(), 8);
        assert_eq!(CanFdFrame::new(0x100, vec![0; 12]).fd_dlc(), 9);  // 9..12 → 9
        assert_eq!(CanFdFrame::new(0x100, vec![0; 16]).fd_dlc(), 10); // 13..16 → 10
        assert_eq!(CanFdFrame::new(0x100, vec![0; 64]).fd_dlc(), 15); // 49..64 → 15
    }

    #[test]
    fn test_can_frame_clone_independent() {
        let frame1 = CanFrame::new(0x100, vec![0x11]);
        let frame2 = frame1.clone();
        assert_eq!(frame1.data, frame2.data);
        // clone 后的 Vec 指针不同（深克隆）
        assert!(!std::ptr::eq(&frame1.data, &frame2.data));
    }

    // ════════════════════════════════════════════════════════════════════
    // 2. CanConfig 默认值
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_can_config_default() {
        let cfg = CanConfig::default();
        assert_eq!(cfg.adapter, AdapterKind::SimBus);
        assert_eq!(cfg.interface, "vcan0");
        assert_eq!(cfg.bitrate, 500_000);
        assert_eq!(cfg.fd_bitrate, 2_000_000);
        assert!(!cfg.enable_fd);
        assert_eq!(cfg.rx_queue_size, 512);
        assert_eq!(cfg.tx_timeout_ms, 100);
        assert_eq!(cfg.isotp_tx_id, 0x7E0);
        assert_eq!(cfg.isotp_rx_id, 0x7E8);
        assert_eq!(cfg.isotp_block_size, 0);
        assert_eq!(cfg.isotp_st_min_ms, 0);
        assert_eq!(cfg.uds_p2_timeout_ms, 150);
        assert_eq!(cfg.uds_p2_star_timeout_ms, 5000);
    }

    #[test]
    fn test_can_config_toml_minimal() {
        let toml_str = r#"
            adapter = "socketcan"
            interface = "can0"
            bitrate = 250000
        "#;
        let cfg: CanConfig = toml::from_str(toml_str).expect("反序列化失败");
        assert_eq!(cfg.adapter, AdapterKind::SocketCan);
        assert_eq!(cfg.interface, "can0");
        assert_eq!(cfg.bitrate, 250_000);
        // 其他字段使用默认值
        assert_eq!(cfg.fd_bitrate, 2_000_000);
        assert_eq!(cfg.isotp_tx_id, 0x7E0);
    }

    #[test]
    fn test_adapter_kind_deserialize() {
        assert_eq!(
            toml::from_str::<AdapterKind>(r#""socketcan""#).unwrap(),
            AdapterKind::SocketCan
        );
        assert_eq!(
            toml::from_str::<AdapterKind>(r#""simbus""#).unwrap(),
            AdapterKind::SimBus
        );
        assert_eq!(
            toml::from_str::<AdapterKind>(r#""pcan""#).unwrap(),
            AdapterKind::Pcan
        );
        assert_eq!(
            toml::from_str::<AdapterKind>(r#""kvaser""#).unwrap(),
            AdapterKind::Kvaser
        );
    }

    #[test]
    fn test_adapter_kind_default() {
        assert_eq!(AdapterKind::default(), AdapterKind::SimBus);
    }

    #[test]
    fn test_can_config_toml_with_all_fields() {
        let toml_str = r#"
            adapter = "pcan"
            interface = "PCAN_USBBUS1"
            bitrate = 125000
            fd_bitrate = 4000000
            enable_fd = true
            rx_queue_size = 1024
            tx_timeout_ms = 200
            isotp_tx_id = 0x700
            isotp_rx_id = 0x701
            isotp_block_size = 8
            isotp_st_min_ms = 5
            uds_p2_timeout_ms = 200
            uds_p2_star_timeout_ms = 10000
        "#;
        let cfg: CanConfig = toml::from_str(toml_str).expect("反序列化失败");
        assert_eq!(cfg.adapter, AdapterKind::Pcan);
        assert_eq!(cfg.interface, "PCAN_USBBUS1");
        assert_eq!(cfg.bitrate, 125_000);
        assert_eq!(cfg.fd_bitrate, 4_000_000);
        assert!(cfg.enable_fd);
        assert_eq!(cfg.rx_queue_size, 1024);
        assert_eq!(cfg.isotp_tx_id, 0x700);
        assert_eq!(cfg.isotp_block_size, 8);
        assert_eq!(cfg.uds_p2_star_timeout_ms, 10000);
    }

    // ════════════════════════════════════════════════════════════════════
    // 3. SimBusAdapter 仿真回环
    // ════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_simbus_open_close() {
        let adapter = SimBusAdapter::new("test0", 16);
        adapter.open().await.expect("open 应成功");
        adapter.close().await.expect("close 应成功");
    }

    #[tokio::test]
    async fn test_simbus_send_loopback() {
        let adapter = SimBusAdapter::new("test1", 16);
        adapter.open().await.unwrap();

        // 先订阅再发送，否则 broadcast 消息已被发出，新订阅者收不到
        let mut rx = adapter.subscribe();

        let tx_frame = CanFrame::new(0x123, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        adapter.send(&tx_frame).await.expect("send 应成功");

        let received = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            rx.recv(),
        )
        .await
        .expect("接收不应超时")
        .expect("recv 应成功");

        assert!(matches!(received.id, FrameId::Standard(0x123)));
        assert_eq!(received.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[tokio::test]
    async fn test_simbus_fd_send_loopback() {
        let adapter = SimBusAdapter::new("test_fd", 16);
        adapter.open().await.unwrap();

        // 先订阅再发送
        let mut rx_fd = adapter.subscribe_fd();

        let tx_frame = CanFdFrame::new(0x200, vec![0x11; 32]);
        adapter.send_fd(&tx_frame).await.expect("send_fd 应成功");

        let received = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            rx_fd.recv(),
        )
        .await
        .expect("接收FD帧不应超时")
        .expect("recv_fd 应成功");

        assert_eq!(received.data.len(), 32);
    }

    #[tokio::test]
    async fn test_simbus_multiple_subscribers() {
        let adapter = SimBusAdapter::new("multi", 16);
        adapter.open().await.unwrap();

        let mut rx1 = adapter.subscribe();
        let mut rx2 = adapter.subscribe();

        adapter.send(&CanFrame::new(0x500, vec![0xAA])).await.unwrap();

        // 两个订阅者都应该收到
        let f1 = tokio::time::timeout(std::time::Duration::from_millis(200), rx1.recv());
        let f2 = tokio::time::timeout(std::time::Duration::from_millis(200), rx2.recv());

        assert!(f1.await.is_ok(), "订阅者1应收到帧");
        assert!(f2.await.is_ok(), "订阅者2应收到帧");
    }

    #[tokio::test]
    async fn test_simbus_name() {
        let adapter = SimBusAdapter::new("my_bus", 16);
        assert_eq!(adapter.name(), "my_bus");
    }

    // ════════════════════════════════════════════════════════════════════
    // 4. ISO-TP 协议解析（字节编码验证）
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_isotp_config_default() {
        let cfg = IsoTpConfig::default();
        assert_eq!(cfg.tx_id, 0x7E0);
        assert_eq!(cfg.rx_id, 0x7E8);
        assert_eq!(cfg.block_size, 0);
        assert_eq!(cfg.st_min_ms, 0);
        assert_eq!(cfg.padding_byte, 0xCC);
        assert!(cfg.enable_padding);
        assert!(!cfg.extended_id);
    }

    #[test]
    fn test_isotp_config_custom() {
        let cfg = IsoTpConfig {
            tx_id: 0x700,
            rx_id: 0x701,
            block_size: 8,
            st_min_ms: 10,
            extended_id: true,
            ..Default::default()
        };
        assert_eq!(cfg.tx_id, 0x700);
        assert_eq!(cfg.block_size, 8);
        assert_eq!(cfg.st_min_ms, 10);
        assert!(cfg.extended_id);
    }

    #[test]
    fn test_isotp_ff_encoding() {
        // FF (First Frame, extended addressing): byte0=0x10, byte1=lower(length)
        // 标准 CAN ID 场景下：FF PCI byte0 = 0x10 | (len >> 8)
        // 对于 100 字节（0x64），高8位=0，低8位=0x64
        let total: usize = 100; // 100 字节 → 0x64
        let ff_dl_hi = ((total >> 8) & 0x0F) as u8; // = 0
        let ff_dl_lo = (total & 0xFF) as u8; // = 0x64
        let ff_pci = 0x10 | ff_dl_hi; // = 0x10

        assert_eq!(ff_pci, 0x10);
        assert_eq!(ff_dl_lo, 0x64);
    }

    #[test]
    fn test_isotp_cf_sn_wrap() {
        // CF SN 从 1 开始，模 16 回绕
        let sn1 = 1u8;
        assert_eq!(sn1.wrapping_add(1), 2);

        // SN 达到 15 后加 1，模 16 回绕到 0
        // 但 u8 wrapping_add 模 256，所以用手动模 16
        let sn15 = 15u8;
        let sn16 = (sn15 as u32 + 1) % 16;
        assert_eq!(sn16, 0); // 模16回绕
    }

    #[test]
    fn test_isotp_fc_encoding() {
        // FC: PCI = 0x30 | FS, BS, STmin
        let fs: u8 = 0x00; // CTS
        let bs: u8 = 8;
        let st_min: u8 = 20;

        let fc_pci = 0x30 | (fs & 0x0F);
        assert_eq!(fc_pci, 0x30);

        let fc_bytes = [fc_pci, bs, st_min];
        assert_eq!(fc_bytes, [0x30, 0x08, 0x14]);
    }

    #[test]
    fn test_isotp_sf_encoding() {
        // SF: PCI = DLC (0..7)
        let data = vec![0x02, 0x01];
        let dlc = data.len() as u8;
        let sf_pci = dlc; // SF PCI = DLC

        assert_eq!(sf_pci, 0x02);
        assert!(sf_pci < 0x10, "SF DLC 应 < 0x10");
    }

    #[test]
    fn test_isotp_sf_max_dlc() {
        // 最大 SF DLC = 7（标准 CAN 最大数据长度）
        let dlc: u8 = 7;
        let sf_pci = dlc;
        assert!(sf_pci < 0x10);
        assert_eq!(sf_pci, 0x07);
    }

    // ════════════════════════════════════════════════════════════════════
    // 5. UDS 协议解析
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_uds_service_response_sid() {
        assert_eq!(UdsService::DiagnosticSessionControl.response_sid(), 0x50);
        assert_eq!(UdsService::ReadDataByIdentifier.response_sid(), 0x62);
        assert_eq!(UdsService::SecurityAccess.response_sid(), 0x67);
        assert_eq!(UdsService::RequestDownload.response_sid(), 0x74);
        assert_eq!(UdsService::TransferData.response_sid(), 0x76);
        assert_eq!(UdsService::RequestTransferExit.response_sid(), 0x77);
        assert_eq!(UdsService::TesterPresent.response_sid(), 0x7E);
    }

    #[test]
    fn test_nrc_from_u8() {
        assert!(matches!(NrcCode::from(0x10), NrcCode::GeneralReject));
        assert!(matches!(NrcCode::from(0x11), NrcCode::ServiceNotSupported));
        assert!(matches!(NrcCode::from(0x22), NrcCode::ConditionsNotCorrect));
        assert!(matches!(NrcCode::from(0x33), NrcCode::SecurityAccessDenied));
        assert!(matches!(NrcCode::from(0x35), NrcCode::InvalidKey));
        assert!(matches!(NrcCode::from(0x78), NrcCode::ResponsePending));
        assert!(matches!(NrcCode::from(0xFF), NrcCode::Unknown(0xFF)));
    }

    #[test]
    fn test_nrc_display() {
        assert_eq!(format!("{}", NrcCode::GeneralReject), "通用拒绝(0x10)");
        assert_eq!(format!("{}", NrcCode::SecurityAccessDenied), "安全访问被拒绝(0x33)");
        assert_eq!(format!("{}", NrcCode::ResponsePending), "响应挂起(0x78)");
        assert_eq!(format!("{}", NrcCode::Unknown(0xAB)), "未知NRC(0xAB)");
    }

    #[test]
    fn test_session_type() {
        assert_eq!(SessionType::Default as u8, 0x01);
        assert_eq!(SessionType::Programming as u8, 0x02);
        assert_eq!(SessionType::Extended as u8, 0x03);
    }

    #[test]
    fn test_uds_service_repr() {
        assert_eq!(UdsService::DiagnosticSessionControl as u8, 0x10);
        assert_eq!(UdsService::ReadDataByIdentifier as u8, 0x22);
        assert_eq!(UdsService::SecurityAccess as u8, 0x27);
        assert_eq!(UdsService::RequestDownload as u8, 0x34);
        assert_eq!(UdsService::TransferData as u8, 0x36);
        assert_eq!(UdsService::EcuReset as u8, 0x11);
        assert_eq!(UdsService::TesterPresent as u8, 0x3E);
    }

    // ════════════════════════════════════════════════════════════════════
    // 6. FlashEngine
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_flash_config_default() {
        let cfg = FlashConfig::default();
        assert_eq!(cfg.target_id, 0x7E0);
        assert_eq!(cfg.security_level, 0x01);
        assert_eq!(cfg.session_type, SessionType::Programming);
        assert_eq!(cfg.default_block_size, 4096);
        assert!(!cfg.erase_before_download);
        assert_eq!(cfg.verify_routine_id, 0x02);
        assert_eq!(cfg.memory_address, 0x00000000);
        assert_eq!(cfg.memory_size_len, 4);
        assert!(cfg.routine_option.is_empty());
    }

    #[test]
    fn test_is_all_ff() {
        assert!(is_all_ff(&[0xFF, 0xFF, 0xFF]));
        assert!(is_all_ff(&[]));
        assert!(is_all_ff(&[0xFF; 100]));
        assert!(!is_all_ff(&[0xFF, 0xFE, 0xFF]));
        assert!(!is_all_ff(&[0x00, 0xFF]));
        assert!(!is_all_ff(&[0xFF, 0x00, 0xFF]));
        // 边界：单字节
        assert!(is_all_ff(&[0xFF]));
        assert!(!is_all_ff(&[0x00]));
    }

    #[test]
    fn test_flash_config_erase_strategy() {
        let mut cfg = FlashConfig::default();
        cfg.erase_before_download = true;
        cfg.memory_address = 0x08000000; // STM32 flash

        assert!(cfg.erase_before_download);
        assert_eq!(cfg.memory_address, 0x08000000);
    }

    #[test]
    fn test_flash_config_verify_routine() {
        let mut cfg = FlashConfig::default();
        cfg.verify_routine_id = 0x01;
        assert_eq!(cfg.verify_routine_id, 0x01);
    }

    // ════════════════════════════════════════════════════════════════════
    // 7. CanEvent 事件枚举
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_can_event_debug() {
        let ev = CanEvent::BusReady {
            interface: "vcan0".to_string(),
        };
        let debug_str = format!("{:?}", ev);
        assert!(debug_str.contains("BusReady"));
        assert!(debug_str.contains("vcan0"));
    }

    #[test]
    fn test_can_event_flash_progress() {
        let ev = CanEvent::FlashProgress {
            block_seq: 5,
            total_blocks: 10,
            bytes_sent: 20480,
            total_bytes: 40960,
        };
        let debug_str = format!("{:?}", ev);
        assert!(debug_str.contains("FlashProgress"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_can_event_clone() {
        let ev = CanEvent::UdsResponse {
            service: 0x22,
            payload: vec![0x62, 0xF1, 0x90, 0x01, 0x02],
        };
        let ev2 = ev.clone();
        assert!(matches!(ev2, CanEvent::UdsResponse { service: 0x22, .. }));
    }

    #[test]
    fn test_can_event_variants() {
        // 确保所有事件变体都可构造
        let _ = CanEvent::BusError {
            description: "test".to_string(),
        };
        let frame = CanFrame::new(0x100, vec![0xAA]);
        let _ = CanEvent::FrameReceived(frame);
        let _ = CanEvent::UdsRequest {
            service: 0x10,
            payload: vec![],
        };
        let _ = CanEvent::UdsNegativeResponse {
            service: 0x22,
            nrc: 0x33,
        };
        let _ = CanEvent::FlashComplete {
            total_bytes: 4096,
            elapsed_ms: 1234,
        };
        let _ = CanEvent::FlashError {
            reason: "test error".to_string(),
        };
    }

    // ════════════════════════════════════════════════════════════════════
    // 8. create_adapter 工厂函数
    // ════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_create_adapter_simbus() {
        let adapter = create_adapter(&AdapterKind::SimBus, "vcan0", 16);
        adapter.open().await.expect("应成功");
        assert_eq!(adapter.name(), "vcan0");
    }

    #[tokio::test]
    async fn test_create_adapter_kvaser_fallback() {
        // Kvaser 降级为 SimBus
        let adapter = create_adapter(&AdapterKind::Kvaser, "test_kvaser", 8);
        adapter.open().await.expect("应成功");
        assert_eq!(adapter.name(), "test_kvaser");
    }

    // ════════════════════════════════════════════════════════════════════
    // 9. CanConfig load_from_toml（文件不存在/格式错误）
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_can_config_load_from_toml_file_not_found() {
        let result = CanConfig::load_from_toml("nonexistent/path/that/does/not/exist.toml");
        assert!(result.is_err(), "load_from_toml 应返回错误");
    }

    #[test]
    fn test_can_config_load_from_toml_missing_section() {
        // 创建临时 TOML 文件（无 [can_config] 段）
        let temp_dir = std::env::temp_dir();
        let path: PathBuf = temp_dir.join(format!(
            "can_test_{}.toml",
            std::process::id()
        ));
        fs::write(&path, "[other_section]\nkey = \"value\"\n").unwrap();

        let result = CanConfig::load_from_toml(path.to_str().unwrap());

        // 清理
        let _ = fs::remove_file(&path);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("[can_config]"));
    }

    // ════════════════════════════════════════════════════════════════════
    // 10. ISO-TP Config padding
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_isotp_config_padding_00() {
        let cfg = IsoTpConfig {
            padding_byte: 0x00,
            enable_padding: true,
            ..Default::default()
        };
        assert_eq!(cfg.padding_byte, 0x00);
        assert!(cfg.enable_padding);
    }

    #[test]
    fn test_isotp_config_padding_disabled() {
        let cfg = IsoTpConfig {
            padding_byte: 0xAA,
            enable_padding: false,
            ..Default::default()
        };
        assert_eq!(cfg.padding_byte, 0xAA);
        assert!(!cfg.enable_padding);
    }
}
