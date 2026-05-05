//! MediaMTX 流媒体后端适配器 未验证
//!
//! [MediaMTX]（原名 rtsp-simple-server）是一个 Go 编写的轻量级媒体服务器，
//! 支持 RTSP、RTMP、HLS、WebRTC 等多种协议，适合轻量部署场景。
//!
//! # 工作原理
//!
//! MediaMTX 不原生支持 GB28181 RTP 接收（没有 `openRtpServer` 这样的 API），
//! 所以本适配器采用以下策略：
//!
//! 1. **RTP 接收**：使用本地 UDP socket 监听，接收设备推来的 RTP 包，
//!    再通过 RTSP 推流到 MediaMTX（`rtsps://...`）。
//!    在生产场景中，通常需要额外配置 FFmpeg 或 rtp2rtsp 组件。
//!
//! 2. **播放分发**：MediaMTX 负责将流转发给多个观看端（RTSP/HLS/WebRTC）。
//!
//! # 配置示例
//!
//! ```toml
//! [gb28181_server_config.media_backend]
//! type = "mediamtx"
//!
//! [gb28181_server_config.media_backend.mediamtx]
//! api_url      = "http://127.0.0.1:9997"   # MediaMTX HTTP API
//! rtsp_url     = "rtsp://127.0.0.1:8554"   # RTSP 基础地址（对外）
//! rtsps_url    = "rtsps://127.0.0.1:8322"  # RTSP over TLS（可选）
//! rtmp_url     = "rtmp://127.0.0.1:1935"   # RTMP 基础地址
//! hls_url      = "http://127.0.0.1:8888"   # HLS 基础地址
//! hls_https_url = "https://127.0.0.1:8889" # HLS over HTTPS（可选）
//! rtp_ip       = "0.0.0.0"                 # 本地 RTP 监听 IP
//! rtp_port_start = 30000                   # RTP 端口范围起始
//! rtp_port_end   = 31000                   # RTP 端口范围结束
//! ```
//!
//! [MediaMTX]: https://github.com/bluenviron/mediamtx

use super::{
    MediaBackend, MediaStreamInfo, OpenRtpRequest, PlayUrls, RtpServerHandle,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};
use tx_di_core::{IE, RIE};
// ── 配置 ─────────────────────────────────────────────────────────────────────

/// MediaMTX 后端配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaMtxConfig {
    /// MediaMTX HTTP API 地址（用于查询/管理路径）
    #[serde(default = "default_api_url")]
    pub api_url: String,

    /// 对外 RTSP 基础地址（设备和用户播放时使用）
    #[serde(default = "default_rtsp_url")]
    pub rtsp_url: String,

    /// 对外 RTSP over TLS 基础地址（空字符串 = 不启用）
    #[serde(default)]
    pub rtsps_url: String,

    /// RTMP 基础地址
    #[serde(default = "default_rtmp_url")]
    pub rtmp_url: String,

    /// HLS 基础地址（http://）
    #[serde(default = "default_hls_url")]
    pub hls_url: String,

    /// HLS over HTTPS 基础地址（空字符串 = 不启用）
    #[serde(default)]
    pub hls_https_url: String,

    /// 本地监听 RTP 包的 IP（0.0.0.0 表示所有接口）
    #[serde(default = "default_rtp_ip")]
    pub rtp_ip: String,

    /// RTP 端口分配起始值
    #[serde(default = "default_rtp_start")]
    pub rtp_port_start: u16,

    /// RTP 端口分配结束值（不含）
    #[serde(default = "default_rtp_end")]
    pub rtp_port_end: u16,

    /// HTTP 请求超时秒数
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for MediaMtxConfig {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            rtsp_url: default_rtsp_url(),
            rtsps_url: String::new(),
            rtmp_url: default_rtmp_url(),
            hls_url: default_hls_url(),
            hls_https_url: String::new(),
            rtp_ip: default_rtp_ip(),
            rtp_port_start: default_rtp_start(),
            rtp_port_end: default_rtp_end(),
            timeout_secs: default_timeout(),
        }
    }
}

fn default_api_url() -> String {
    "http://127.0.0.1:9997".to_string()
}
fn default_rtsp_url() -> String {
    "rtsp://127.0.0.1:8554".to_string()
}
fn default_rtmp_url() -> String {
    "rtmp://127.0.0.1:1935".to_string()
}
fn default_hls_url() -> String {
    "http://127.0.0.1:8888".to_string()
}
fn default_rtp_ip() -> String {
    "0.0.0.0".to_string()
}
fn default_rtp_start() -> u16 {
    30000
}
fn default_rtp_end() -> u16 {
    31000
}
fn default_timeout() -> u64 {
    10
}

// ── MediaMTX API 数据结构 ─────────────────────────────────────────────────────

/// MediaMTX API: /v3/paths/list 响应
#[derive(Debug, Deserialize)]
struct MtxPathListResponse {
    items: Vec<MtxPathItem>,
}

/// 单条路径信息
#[derive(Debug, Deserialize)]
struct MtxPathItem {
    name: String,
    source: Option<MtxSource>,
    readers: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct MtxSource {
    #[serde(rename = "type")]
    source_type: String,
}

// ── 适配器 ────────────────────────────────────────────────────────────────────

/// 正在监听的 RTP 端口条目
struct RtpEntry {
    port: u16,
    // 未来可加 UdpSocket 句柄或 JoinHandle
}

/// MediaMTX 流媒体后端
///
/// 通过本地 UDP socket 管理 RTP 端口，并将流通过 RTSP 推送到 MediaMTX。
pub struct MediaMtxBackend {
    config: MediaMtxConfig,
    http: reqwest::Client,
    /// 已分配的 RTP 端口表（stream_id → RtpEntry）
    rtp_ports: Arc<DashMap<String, RtpEntry>>,
    /// 下一个候选端口（轮转分配）
    next_port: Arc<AtomicU16>,
}

impl MediaMtxBackend {
    /// 创建新的 MediaMTX 后端实例
    pub fn new(config: MediaMtxConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("创建 HTTP 客户端失败");

        let start_port = config.rtp_port_start;

        Self {
            config,
            http,
            rtp_ports: Arc::new(DashMap::new()),
            next_port: Arc::new(AtomicU16::new(start_port)),
        }
    }

    /// 分配一个空闲的 UDP 端口（从配置范围中轮转）
    fn allocate_port(&self) -> anyhow::Result<u16> {
        let start = self.config.rtp_port_start;
        let end = self.config.rtp_port_end;
        let range = (end - start) as u32;

        for _ in 0..range {
            let candidate = self.next_port.fetch_add(1, Ordering::SeqCst);
            // 超出范围时回绕
            let port = start + (candidate % range as u16);

            // 尝试绑定，验证端口可用
            if self.rtp_ports.len() < range as usize {
                if let Ok(_sock) = UdpSocket::bind(format!("{}:{}", self.config.rtp_ip, port)) {
                    return Ok(port);
                }
            }
        }

        Err(anyhow::anyhow!(
            "MediaMTX 后端：端口范围 {}-{} 内没有可用 UDP 端口",
            start,
            end
        ))
    }

    /// 调用 MediaMTX API
    async fn api_get<T: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
    ) -> anyhow::Result<T> {
        let url = format!("{}{}", self.config.api_url, path);
        debug!(url = %url, "MediaMTX API 请求");
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("MediaMTX API 请求失败 [{}]: {}", path, e))?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "MediaMTX API 返回 {} [{}]",
                resp.status(),
                path
            ));
        }

        resp.json::<T>()
            .await
            .map_err(|e| anyhow::anyhow!("MediaMTX 响应解析失败: {}", e))
    }
}


#[async_trait::async_trait]
impl MediaBackend for MediaMtxBackend {
    async fn open_rtp_server(&self, req: OpenRtpRequest) -> RIE<RtpServerHandle> {
        let port = if req.port != 0 {
            req.port
        } else {
            self.allocate_port().map_err(|e| IE::Other(e.to_string()))?
        };

        self.rtp_ports
            .insert(req.stream_id.clone(), RtpEntry { port });

        debug!(
            stream_id = %req.stream_id,
            port = port,
            "MediaMTX 后端：分配 RTP 端口"
        );

        Ok(RtpServerHandle {
            stream_id: req.stream_id.clone(),
            port,
            token: Some(format!("rtp/{}", req.stream_id)),
        })
    }

    async fn close_rtp_server(&self, stream_id: &str) -> RIE<()> {
        if self.rtp_ports.remove(stream_id).is_none() {
            warn!(stream_id = %stream_id, "MediaMTX 后端：未找到对应 RTP 端口记录");
        }

        let path = format!("/v3/config/paths/remove/rtp%2F{}", stream_id);
        let url = format!("{}{}", self.config.api_url, path);
        let _ = self.http.delete(&url).send().await;

        Ok(())
    }

    async fn is_stream_online(&self, stream_id: &str) -> bool {
        let path = format!("/v3/paths/get/rtp%2F{}", stream_id);
        match self.api_get::<serde_json::Value>(&path).await {
            Ok(val) => val.get("source").is_some(),
            Err(_) => false,
        }
    }

    fn get_play_urls(&self, stream_id: &str) -> PlayUrls {
        let path = format!("rtp/{}", stream_id);

        let rtsps = if !self.config.rtsps_url.is_empty() {
            format!("{}/{}", self.config.rtsps_url, path)
        } else {
            String::new()
        };

        let hls_https = if !self.config.hls_https_url.is_empty() {
            format!("{}/{}", self.config.hls_https_url, path)
        } else {
            String::new()
        };

        PlayUrls {
            stream_id: stream_id.to_string(),
            rtsp: format!("{}/{}", self.config.rtsp_url, path),
            rtsps,
            rtmp: format!("{}/{}", self.config.rtmp_url, path),
            hls: format!("{}/{}", self.config.hls_url, path),
            hls_https,
            webrtc: format!(
                "{}/webrtc/{}",
                self.config
                    .hls_url
                    .replace("http://", "webrtc://")
                    .replace("https://", "webrtcs://"),
                path
            ),
            flv: String::new(),
            flv_https: String::new(),
        }
    }

    async fn list_streams(&self) -> RIE<Vec<MediaStreamInfo>> {
        let resp: MtxPathListResponse = self
            .api_get("/v3/paths/list")
            .await
            .map_err(|e| IE::Other(e.to_string()))?;

        Ok(resp
            .items
            .into_iter()
            .filter(|p| p.source.is_some())
            .map(|p| {
                let reader_count = p.readers.as_ref().map(|r| r.len() as u32).unwrap_or(0);
                MediaStreamInfo {
                    app: "rtp".to_string(),
                    stream_id: p.name.clone(),
                    schema: p
                        .source
                        .as_ref()
                        .map(|s| s.source_type.clone())
                        .unwrap_or_default(),
                    reader_count,
                    alive_secs: 0,
                    origin_url: None,
                }
            })
            .collect())
    }

    fn backend_name(&self) -> &'static str {
        "mediamtx"
    }

    async fn health_check(&self) -> RIE<()> {
        let url = format!("{}/v3/config/global/get", self.config.api_url);
        self.http
            .get(&url)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| IE::Other(e.to_string()))
    }
}
