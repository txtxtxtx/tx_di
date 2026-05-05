//! Null 流媒体后端（测试/开发用）
//!
//! 所有操作均立即成功，但不做任何实际媒体处理。
//! 适合以下场景：
//! - 单元测试（无需运行 ZLM 或 MediaMTX）
//! - 开发阶段验证 SIP 信令流程
//! - CI/CD 环境（不依赖外部服务）
//!
//! # 行为说明
//!
//! | 方法                | 行为                          |
//! |--------------------|-------------------------------|
//! | `open_rtp_server`  | 成功，返回端口 0               |
//! | `close_rtp_server` | 成功，无操作                   |
//! | `is_stream_online` | 始终返回 `false`              |
//! | `get_play_urls`    | 返回空 URL                    |
//! | `list_streams`     | 返回空列表                     |

use tx_di_core::RIE;
use super::{MediaBackend, MediaStreamInfo, OpenRtpRequest, PlayUrls, RtpServerHandle};

/// 空流媒体后端（不做实际媒体操作）
#[derive(Debug, Clone, Copy, Default)]
pub struct NullBackend;

#[async_trait::async_trait]
impl MediaBackend for NullBackend {
    async fn open_rtp_server(&self, req: OpenRtpRequest) -> RIE<RtpServerHandle> {
        Ok(RtpServerHandle {
            stream_id: req.stream_id,
            port: req.port,
            token: None,
        })
    }

    async fn close_rtp_server(&self, _stream_id: &str) -> RIE<()> {
        Ok(())
    }

    async fn is_stream_online(&self, _stream_id: &str) -> bool {
        false
    }

    fn get_play_urls(&self, stream_id: &str) -> PlayUrls {
        PlayUrls::empty(stream_id)
    }

    async fn list_streams(&self) -> RIE<Vec<MediaStreamInfo>> {
        Ok(vec![])
    }

    fn backend_name(&self) -> &'static str {
        "null"
    }

    async fn health_check(&self) -> RIE<()> {
        Ok(())
    }
}
