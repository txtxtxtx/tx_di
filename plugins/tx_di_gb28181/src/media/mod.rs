//! 统一流媒体后端抽象接口
//!
//! 本模块定义了 [`MediaBackend`] trait，使 GB28181 服务端可无缝对接多种流媒体服务：
//!
//! | 后端             | 特点                         | 说明                                     |
//! |-----------------|------------------------------|------------------------------------------|
//! | [`ZlmBackend`]  | ZLMediaServer（首选）         | 国内主流，HTTP API，支持 GB28181 原生收流  |
//! | [`MediaMtxBackend`] | MediaMTX（前身 rtsp-simple-server）| Go 实现，轻量，通过 RTSP 推流     |
//! | [`NullBackend`] | 空实现（测试/开发用）          | 所有操作均成功，不做实际媒体操作           |
//!
//! # 快速开始
//!
//! 在 TOML 配置中选择后端：
//!
//! ```toml
//! [gb28181_server_config.media_backend]
//! type = "zlm"                          # zlm | mediamtx | null
//!
//! [gb28181_server_config.media_backend.zlm]
//! base_url    = "http://127.0.0.1:8080"
//! secret      = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
//! rtsp_port   = 554                      # RTSP 端口
//! rtsps_port  = 322                      # RTSP over TLS 端口（0=不启用）
//! rtmp_port   = 1935                     # RTMP 端口
//! http_port   = 8080                     # HTTP API 端口（0=自动推断）
//! https_port  = 8443                     # HTTPS 端口（0=不启用）
//!
//! [gb28181_server_config.media_backend.mediamtx]
//! api_url       = "http://127.0.0.1:9997"
//! rtsp_url      = "rtsp://127.0.0.1:8554"
//! rtsps_url     = "rtsps://127.0.0.1:8322"   # 可选
//! rtmp_url      = "rtmp://127.0.0.1:1935"
//! hls_url       = "http://127.0.0.1:8888"
//! hls_https_url = "https://127.0.0.1:8889"   # 可选
//! ```
//!
//! # 自定义后端示例
//!
//! ```rust,ignore
//! use tx_di_gb28181::media::{MediaBackend, RtpServerHandle, PlayUrls};
//! use async_trait::async_trait;
//! use tx_di_core::RIE;
//!
//! struct MyBackend;
//!
//! #[async_trait]
//! impl MediaBackend for MyBackend {
//!     async fn open_rtp_server(&self, req: OpenRtpRequest) -> RIE<RtpServerHandle> {
//!         // 返回自定义端口
//!         Ok(RtpServerHandle { stream_id: req.stream_id, port: 10000, token: None })
//!     }
//!     async fn close_rtp_server(&self, stream_id: &str) -> RIE<()> { Ok(()) }
//!     async fn is_stream_online(&self, stream_id: &str) -> bool { false }
//!     fn get_play_urls(&self, stream_id: &str) -> PlayUrls {
//!         PlayUrls::empty(stream_id)
//!     }
//!     fn backend_name(&self) -> &'static str { "my-backend" }
//! }
//! ```

pub mod mediamtx;
pub mod null;
pub mod zlm;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

// 子模块再导出
pub use mediamtx::MediaMtxBackend;
pub use null::NullBackend;
use tx_di_core::RIE;
pub use zlm::ZlmBackend;

// ── 公共数据结构 ────────────────────────────────────────────────────────────

/// 打开 RTP 接收端口的请求参数
#[derive(Debug, Clone)]
pub struct OpenRtpRequest {
    /// 流的逻辑标识符（例如 "channel_id_sn"）
    pub stream_id: String,

    /// 希望绑定的端口（0 = 由后端自动分配）
    pub port: u16,

    /// 传输协议模式
    pub tcp_mode: TcpMode,

    /// 超时秒数（0 = 不超时）
    pub timeout_secs: u32,
}

impl OpenRtpRequest {
    /// 创建标准 UDP 请求（端口自动分配）
    pub fn udp(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            port: 0,
            tcp_mode: TcpMode::Udp,
            timeout_secs: 0,
        }
    }

    /// 创建指定端口的 UDP 请求
    pub fn udp_with_port(stream_id: impl Into<String>, port: u16) -> Self {
        Self {
            stream_id: stream_id.into(),
            port,
            tcp_mode: TcpMode::Udp,
            timeout_secs: 0,
        }
    }

    /// 创建 TCP 被动模式请求
    pub fn tcp_passive(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            port: 0,
            tcp_mode: TcpMode::TcpPassive,
            timeout_secs: 0,
        }
    }
}

/// RTP 传输协议模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TcpMode {
    /// UDP 模式（默认，GB28181 最常用）
    Udp,
    /// TCP 主动模式（服务端主动连接设备）
    TcpActive,
    /// TCP 被动模式（服务端监听，设备主动连接）
    TcpPassive,
}

impl TcpMode {
    /// 转换为 ZLM API 的数值
    pub fn as_zlm_value(self) -> u8 {
        match self {
            TcpMode::Udp => 0,
            TcpMode::TcpActive => 1,
            TcpMode::TcpPassive => 2,
        }
    }
}

/// 已开启的 RTP 接收端口句柄
#[derive(Debug, Clone)]
pub struct RtpServerHandle {
    /// 流标识（与请求保持一致）
    pub stream_id: String,

    /// 实际监听的端口
    pub port: u16,

    /// 后端自定义令牌（关闭时透传给后端）
    /// 例如 ZLM 中可不使用，MediaMTX 中可存放 path
    pub token: Option<String>,
}

/// 各协议播放 URL 集合
///
/// 包含明文和加密（TLS/HTTPS）两种版本的地址。
/// 当后端未配置 TLS 时，对应的 `_tls` 字段为空字符串。
#[derive(Debug, Clone, Default)]
pub struct PlayUrls {
    /// 流标识
    pub stream_id: String,

    /// RTSP 播放地址（明文）
    pub rtsp: String,

    /// RTSP over TLS 播放地址（`rtsps://`）
    pub rtsps: String,

    /// RTMP 推流/播放地址（明文）
    pub rtmp: String,

    /// HLS 地址（`http://...m3u8`）
    pub hls: String,

    /// HLS over HTTPS 地址（`https://...m3u8`）
    pub hls_https: String,

    /// WebRTC 地址
    pub webrtc: String,

    /// HTTP-FLV 地址（明文）
    pub flv: String,

    /// HTTPS-FLV 地址
    pub flv_https: String,
}

impl PlayUrls {
    /// 创建空 URL 集合（NullBackend 使用）
    pub fn empty(stream_id: &str) -> Self {
        Self {
            stream_id: stream_id.to_string(),
            ..Default::default()
        }
    }

    /// 判断是否有任何可用的播放地址
    pub fn has_any(&self) -> bool {
        !self.rtsp.is_empty()
            || !self.rtsps.is_empty()
            || !self.rtmp.is_empty()
            || !self.hls.is_empty()
            || !self.hls_https.is_empty()
            || !self.webrtc.is_empty()
            || !self.flv.is_empty()
            || !self.flv_https.is_empty()
    }

    /// 返回最优先的播放地址
    ///
    /// 优先级顺序：`RTSP > RTSPS > HLS > HLS-HTTPS > FLV > FLV-HTTPS > RTMP > WebRTC`
    pub fn best(&self) -> Option<&str> {
        if !self.rtsp.is_empty() {
            Some(&self.rtsp)
        } else if !self.rtsps.is_empty() {
            Some(&self.rtsps)
        } else if !self.hls.is_empty() {
            Some(&self.hls)
        } else if !self.hls_https.is_empty() {
            Some(&self.hls_https)
        } else if !self.flv.is_empty() {
            Some(&self.flv)
        } else if !self.flv_https.is_empty() {
            Some(&self.flv_https)
        } else if !self.rtmp.is_empty() {
            Some(&self.rtmp)
        } else if !self.webrtc.is_empty() {
            Some(&self.webrtc)
        } else {
            None
        }
    }

    /// 返回所有非空播放地址（协议名, URL）的列表
    pub fn all(&self) -> Vec<(&str, &str)> {
        let pairs: [(&str, &str); 8] = [
            ("rtsp", &self.rtsp),
            ("rtsps", &self.rtsps),
            ("rtmp", &self.rtmp),
            ("hls", &self.hls),
            ("hls_https", &self.hls_https),
            ("flv", &self.flv),
            ("flv_https", &self.flv_https),
            ("webrtc", &self.webrtc),
        ];
        pairs
            .into_iter()
            .filter(|(_, url)| !url.is_empty())
            .collect()
    }
}

impl fmt::Display for PlayUrls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PlayUrls {{ stream={}, ", self.stream_id)?;
        let mut first = true;
        for (proto, url) in self.all() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}={}", proto, url)?;
            first = false;
        }
        write!(f, " }}")
    }
}

/// 媒体流信息（查询活跃流时返回）
#[derive(Debug, Clone)]
pub struct MediaStreamInfo {
    /// 应用名（例如 "rtp"）
    pub app: String,

    /// 流 ID
    pub stream_id: String,

    /// 播放协议（"rtsp" / "rtmp" / "hls" 等）
    pub schema: String,

    /// 在线观看人数
    pub reader_count: u32,

    /// 流已在线时长（秒）
    pub alive_secs: u64,

    /// 来源 URL（拉流时有效）
    pub origin_url: Option<String>,
}

/// 推流代理信息（addStreamProxy 时返回）
#[derive(Debug, Clone)]
pub struct StreamProxyHandle {
    /// 代理 key（用于删除时传回）
    pub key: String,

    /// 本地流 ID
    pub stream_id: String,
}

// ── 核心 Trait ──────────────────────────────────────────────────────────────

/// 统一流媒体后端接口
///
/// 任何流媒体服务只需实现此 trait 即可接入 GB28181 服务端。
/// 所有方法均为 async，支持 tokio 运行时。
///
/// # 线程安全
///
/// 要求 `Send + Sync + 'static`，可以安全地在 `Arc<dyn MediaBackend>` 中共享。
#[async_trait::async_trait]
pub trait MediaBackend: Send + Sync + 'static {
    // ── 核心 RTP 管理 ─────────────────────────────────────────────────────

    /// 开启 RTP 接收端口
    ///
    /// GB28181 设备收到 INVITE 后会将码流推送到此端口。
    /// 返回包含实际分配端口的句柄，后续需调用 [`close_rtp_server`] 释放。
    ///
    /// [`close_rtp_server`]: MediaBackend::close_rtp_server
    async fn open_rtp_server(&self, req: OpenRtpRequest) -> RIE<RtpServerHandle>;

    /// 关闭 RTP 接收端口并释放资源
    ///
    /// `stream_id` 为 [`open_rtp_server`] 中传入的 `req.stream_id`。
    ///
    /// [`open_rtp_server`]: MediaBackend::open_rtp_server
    async fn close_rtp_server(&self, stream_id: &str) -> RIE<()>;

    /// 查询指定流是否在线（有设备正在推流）
    async fn is_stream_online(&self, stream_id: &str) -> bool;

    /// 获取流的各协议播放 URL
    ///
    /// 调用此方法不需要流当前在线——URL 是静态推导的。
    /// 若要确认流是否可播，先调用 [`is_stream_online`]。
    ///
    /// [`is_stream_online`]: MediaBackend::is_stream_online
    fn get_play_urls(&self, stream_id: &str) -> PlayUrls;

    // ── 扩展功能（默认为空实现，子后端按需覆盖）────────────────────────────

    /// 获取所有活跃媒体流列表
    ///
    /// 默认实现返回空列表（不支持此功能的后端直接使用默认值）。
    async fn list_streams(&self) -> RIE<Vec<MediaStreamInfo>> {
        Ok(vec![])
    }

    /// 添加拉流代理（让媒体服务器主动拉外部 RTSP/RTMP 流）
    ///
    /// 默认实现返回 `Err`（不支持的后端）。
    async fn add_stream_proxy(
        &self,
        _stream_id: &str,
        _source_url: &str,
    ) -> RIE<StreamProxyHandle> {
        Err("当前流媒体后端不支持拉流代理".into())
    }

    /// 删除拉流代理
    ///
    /// 默认实现返回 `Ok(false)`（不支持的后端）。
    async fn remove_stream_proxy(&self, _key: &str) -> RIE<bool> {
        Ok(false)
    }

    /// 后端名称（用于日志和调试）
    fn backend_name(&self) -> &'static str;

    /// 健康检查（检查后端服务是否可达）
    ///
    /// 默认实现始终返回 `Ok(())`。
    async fn health_check(&self) -> RIE<()> {
        Ok(())
    }
}

// ── 配置 & 工厂 ─────────────────────────────────────────────────────────────

/// 流媒体后端类型选择
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// ZLMediaServer（默认）
    #[default]
    Zlm,
    /// MediaMTX (rtsp-simple-server)
    MediaMtx,
    /// 空后端（测试/开发）
    Null,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendType::Zlm => write!(f, "ZLMediaServer"),
            BackendType::MediaMtx => write!(f, "MediaMTX"),
            BackendType::Null => write!(f, "Null"),
        }
    }
}

/// 统一流媒体后端配置
///
/// 在 GB28181 配置文件中通过 `[gb28181_server_config.media_backend]` 指定。
///
/// ```toml
/// [gb28181_server_config.media_backend]
/// type = "zlm"
///
/// [gb28181_server_config.media_backend.zlm]
/// base_url = "http://127.0.0.1:8080"
/// secret   = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
///
/// # 或者使用 MediaMTX
/// # [gb28181_server_config.media_backend]
/// # type = "mediamtx"
/// #
/// # [gb28181_server_config.media_backend.mediamtx]
/// # api_url  = "http://127.0.0.1:9997"
/// # rtsp_url = "rtsp://127.0.0.1:8554"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaBackendConfig {
    /// 后端类型（默认 ZLM）
    #[serde(default, rename = "type")]
    pub backend_type: BackendType,

    /// ZLM 后端配置
    #[serde(default)]
    pub zlm: zlm::ZlmBackendConfig,

    /// MediaMTX 后端配置
    #[serde(default)]
    pub mediamtx: mediamtx::MediaMtxConfig,
}

impl Default for MediaBackendConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Zlm,
            zlm: zlm::ZlmBackendConfig::default(),
            mediamtx: mediamtx::MediaMtxConfig::default(),
        }
    }
}

/// 工厂函数：根据配置构建对应的后端实例
///
/// 返回 `Arc<dyn MediaBackend>` 方便在多处共享。
///
/// # 示例
///
/// ```rust,ignore
/// use tx_di_gb28181::media::{MediaBackendConfig, build_backend};
///
/// let config = MediaBackendConfig::default(); // 默认使用 ZLM
/// let backend = build_backend(&config);
/// let handle = backend.open_rtp_server(OpenRtpRequest::udp("stream_001")).await?;
/// println!("RTP 端口: {}", handle.port);
/// ```
pub fn build_backend(config: &MediaBackendConfig) -> Arc<dyn MediaBackend> {
    match config.backend_type {
        BackendType::Zlm => {
            tracing::info!(
                url = %config.zlm.base_url,
                "🎬 使用 ZLMediaServer 流媒体后端"
            );
            Arc::new(ZlmBackend::new(config.zlm.clone()))
        }
        BackendType::MediaMtx => {
            tracing::info!(
                api = %config.mediamtx.api_url,
                "🎬 使用 MediaMTX 流媒体后端"
            );
            Arc::new(MediaMtxBackend::new(config.mediamtx.clone()))
        }
        BackendType::Null => {
            tracing::warn!("🎬 使用 Null 流媒体后端（测试模式，不会实际处理媒体流）");
            Arc::new(NullBackend)
        }
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_rtp_request_helpers() {
        let req = OpenRtpRequest::udp("stream_1");
        assert_eq!(req.stream_id, "stream_1");
        assert_eq!(req.port, 0);
        assert_eq!(req.tcp_mode, TcpMode::Udp);

        let req2 = OpenRtpRequest::udp_with_port("stream_2", 10000);
        assert_eq!(req2.port, 10000);
        assert_eq!(TcpMode::Udp.as_zlm_value(), 0);
        assert_eq!(TcpMode::TcpActive.as_zlm_value(), 1);
        assert_eq!(TcpMode::TcpPassive.as_zlm_value(), 2);
    }

    #[test]
    fn play_urls_best() {
        let mut urls = PlayUrls::empty("test");
        assert!(urls.best().is_none());
        assert!(!urls.has_any());

        urls.rtsp = "rtsp://localhost/test".to_string();
        assert_eq!(urls.best(), Some("rtsp://localhost/test"));
        assert!(urls.has_any());

        // 优先级：RTSPS > HLS
        let mut urls2 = PlayUrls::empty("test2");
        urls2.rtsps = "rtsps://localhost/test2".to_string();
        urls2.hls = "http://localhost/test2.m3u8".to_string();
        assert_eq!(urls2.best(), Some("rtsps://localhost/test2"));

        // 优先级：HLS > HLS-HTTPS > FLV > FLV-HTTPS
        let mut urls3 = PlayUrls::empty("test3");
        urls3.hls_https = "https://localhost/test3.m3u8".to_string();
        urls3.flv = "http://localhost/test3.flv".to_string();
        assert_eq!(urls3.best(), Some("https://localhost/test3.m3u8"));

        // 空的返回 None
        let empty = PlayUrls::empty("none");
        assert!(!empty.has_any());
        assert!(empty.all().is_empty());
    }

    #[test]
    fn play_urls_all() {
        let mut urls = PlayUrls::empty("test");
        urls.rtsp = "rtsp://a".to_string();
        urls.hls = "http://b".to_string();
        urls.flv_https = "https://c".to_string();
        let all = urls.all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], ("rtsp", "rtsp://a"));
        assert_eq!(all[1], ("hls", "http://b"));
        assert_eq!(all[2], ("flv_https", "https://c"));
    }

    #[test]
    fn play_urls_display() {
        let mut urls = PlayUrls::empty("s1");
        urls.rtsp = "rtsp://a".to_string();
        urls.rtsps = "rtsps://b".to_string();
        urls.hls = "http://c".to_string();
        let display = format!("{}", urls);
        assert!(display.contains("rtsp=rtsp://a"));
        assert!(display.contains("rtsps=rtsps://b"));
        assert!(display.contains("hls=http://c"));
        assert!(display.contains("stream=s1"));
    }

    #[test]
    fn backend_type_default() {
        let cfg = MediaBackendConfig::default();
        assert_eq!(cfg.backend_type, BackendType::Zlm);
    }

    #[tokio::test]
    async fn null_backend_works() {
        let backend = NullBackend;
        let handle = backend
            .open_rtp_server(OpenRtpRequest::udp("test"))
            .await
            .unwrap();
        assert_eq!(handle.stream_id, "test");
        assert_eq!(handle.port, 0);
        assert!(!backend.is_stream_online("test").await);
        backend.close_rtp_server("test").await.unwrap();
        assert_eq!(backend.backend_name(), "null");
    }
}
