//! ZLMediaServer 流媒体后端适配器
//!
//! 本模块包含：
//! - [`ZlmClient`] — ZLM HTTP API 底层客户端
//! - [`ZlmBackendConfig`] — ZLM 后端配置（支持 TLS 端口）
//! - [`ZlmBackend`] — 实现 [`MediaBackend`] trait 的适配器
//!
//! ZLM 是 GB28181 平台的首选流媒体后端：
//! - 原生支持 RTP 接收（openRtpServer）
//! - 支持 RTSP/RTMP/HLS/WebRTC/FLV 多协议输出
//! - HTTP API 简单稳定

use super::{
    MediaBackend, MediaStreamInfo, OpenRtpRequest, PlayUrls, RtpServerHandle, StreamProxyHandle,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use tx_di_core::RIE;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ZLM 配置
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// ZLM 后端配置（同时作为外部 TOML 配置和内部运行配置使用）
///
/// ```toml
/// [gb28181_server_config.media_backend.zlm]
/// base_url    = "http://127.0.0.1:8080"
/// secret      = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
/// rtsp_port   = 554
/// rtsps_port  = 0        # 0 = 不启用
/// rtmp_port   = 1935
/// http_port   = 8080     # 0 = 从 base_url 推断
/// https_port  = 0        # 0 = 不启用
/// timeout_secs = 10
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZlmBackendConfig {
    /// ZLM HTTP API 基础地址，例如 "http://127.0.0.1:8080"
    #[serde(default = "default_zlm_url")]
    pub base_url: String,

    /// ZLM API 鉴权 secret
    #[serde(default = "default_zlm_secret")]
    pub secret: String,

    /// 请求超时秒数（默认 10 秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// RTSP 端口（用于构造播放 URL，默认 554）
    #[serde(default = "default_rtsp_port")]
    pub rtsp_port: u16,

    /// RTSP over TLS 端口（0 = 不启用）
    #[serde(default)]
    pub rtsps_port: u16,

    /// RTMP 端口（默认 1935）
    #[serde(default = "default_rtmp_port")]
    pub rtmp_port: u16,

    /// HTTP API 端口（0 = 从 base_url 推断）
    #[serde(default)]
    pub http_port: u16,

    /// HTTPS 端口（0 = 不启用）
    #[serde(default)]
    pub https_port: u16,
}

impl Default for ZlmBackendConfig {
    fn default() -> Self {
        Self {
            base_url: default_zlm_url(),
            secret: default_zlm_secret(),
            timeout_secs: default_timeout(),
            rtsp_port: default_rtsp_port(),
            rtsps_port: 0,
            rtmp_port: default_rtmp_port(),
            http_port: 0,
            https_port: 0,
        }
    }
}

fn default_zlm_url() -> String {
    "http://127.0.0.1:8080".to_string()
}
fn default_zlm_secret() -> String {
    "035c73f7-bb6b-4889-a715-d9eb2d1925cc".to_string()
}
fn default_timeout() -> u64 {
    10
}
fn default_rtsp_port() -> u16 {
    554
}
fn default_rtmp_port() -> u16 {
    1935
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ZLM HTTP API 数据结构
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 活跃媒体流信息（ZLM getMediaList 返回）
#[derive(Debug, Clone, Deserialize)]
pub struct MediaInfo {
    pub app: String,
    pub stream: String,
    pub schema: String,
    pub vhost: String,
    #[serde(default)]
    pub originUrl: Option<String>,
    #[serde(default)]
    pub aliveSecond: Option<u64>,
    #[serde(default)]
    pub totalReaderCount: Option<u32>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ZLM HTTP API 客户端
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// ZLMediaServer HTTP API 客户端
///
/// 封装 ZLM REST API，所有方法均为 async。
#[derive(Clone)]
pub struct ZlmClient {
    config: ZlmBackendConfig,
    http: reqwest::Client,
}

impl ZlmClient {
    pub fn new(config: ZlmBackendConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("创建 HTTP 客户端失败");
        Self { config, http }
    }

    // ── 内部工具 ─────────────────────────────────────────────────────────────

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    // ── 公开 API ─────────────────────────────────────────────────────────────

    /// 开启 RTP 接收端口
    ///
    /// # 参数
    /// - `stream_id`：流标识，建议用 `channel_id` 或 `call_id`
    /// - `port`：指定端口（0 表示 ZLM 随机分配）
    /// - `tcp_mode`：0=UDP，1=TCP 主动，2=TCP 被动
    pub async fn open_rtp_server(
        &self,
        stream_id: &str,
        port: u16,
        tcp_mode: u8,
    ) -> anyhow::Result<u16> {
        let port_str = port.to_string();
        let tcp_str = tcp_mode.to_string();

        let val = self
            .get_raw(
                "/index/api/openRtpServer",
                &[
                    ("port", &port_str),
                    ("tcp_mode", &tcp_str),
                    ("stream_id", stream_id),
                ],
            )
            .await?;

        let p = val
            .get("port")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("ZLM openRtpServer 响应缺少 port 字段"))?;

        Ok(p as u16)
    }

    /// 关闭 RTP 接收端口
    pub async fn close_rtp_server(&self, stream_id: &str) -> anyhow::Result<bool> {
        let val = self
            .get_raw("/index/api/closeRtpServer", &[("stream_id", stream_id)])
            .await?;

        let hit = val.get("hit").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(hit > 0)
    }

    /// 查询媒体流列表
    pub async fn get_media_list(
        &self,
        app: &str,
        stream: &str,
    ) -> anyhow::Result<Vec<MediaInfo>> {
        let val = self
            .get_raw("/index/api/getMediaList", &[("app", app), ("stream", stream)])
            .await?;

        let arr = val
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let result: Vec<MediaInfo> = arr
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .collect();

        Ok(result)
    }

    /// 添加 RTSP/RTMP 拉流代理
    pub async fn add_stream_proxy(
        &self,
        app: &str,
        stream: &str,
        url: &str,
    ) -> anyhow::Result<String> {
        let val = self
            .get_raw(
                "/index/api/addStreamProxy",
                &[
                    ("vhost", "__defaultVhost__"),
                    ("app", app),
                    ("stream", stream),
                    ("url", url),
                    ("retry_count", "-1"),
                    ("rtp_type", "0"),
                ],
            )
            .await?;

        let key = val
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("ZLM addStreamProxy 响应缺少 key 字段"))?
            .to_string();

        Ok(key)
    }

    /// 删除拉流代理
    pub async fn del_stream_proxy(&self, key: &str) -> anyhow::Result<bool> {
        let val = self
            .get_raw("/index/api/delStreamProxy", &[("key", key)])
            .await?;

        let data = val
            .get("data")
            .and_then(|v| v.get("flag"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(data)
    }

    /// 获取 RTP 推流状态（是否有流在线）
    pub async fn is_stream_online(&self, app: &str, stream: &str) -> bool {
        match self.get_media_list(app, stream).await {
            Ok(list) => !list.is_empty(),
            Err(e) => {
                warn!(error = %e, "ZLM 查询流状态失败");
                false
            }
        }
    }

    // ── 低层 GET ─────────────────────────────────────────────────────────────

    async fn get_raw(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> anyhow::Result<serde_json::Value> {
        let url = self.url(path);
        let mut all_params: Vec<(String, String)> =
            vec![("secret".into(), self.config.secret.clone())];
        for (k, v) in params {
            all_params.push((k.to_string(), v.to_string()));
        }

        debug!(url = %url, ?all_params, "ZLM 请求");

        let resp = self
            .http
            .get(&url)
            .query(&all_params)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("ZLM HTTP 请求失败 [{}]: {}", path, e))?;

        let text = resp
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("ZLM 读响应失败: {}", e))?;

        debug!(response = %text, "ZLM 原始响应");

        let val: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("ZLM JSON 解析失败: {} — 原始: {}", e, text))?;

        let code = val.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = val
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow::anyhow!(
                "ZLM API 错误 [{}] (code={}, msg={})",
                path,
                code,
                msg
            ));
        }

        Ok(val)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MediaBackend 适配器
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// ZLMediaServer 流媒体后端
///
/// 通过 ZLM HTTP API 管理 RTP 端口和媒体流。
pub struct ZlmBackend {
    config: ZlmBackendConfig,
    client: ZlmClient,
}

impl ZlmBackend {
    /// 用给定配置创建 ZLM 后端实例
    pub fn new(config: ZlmBackendConfig) -> Self {
        let client = ZlmClient::new(config.clone());
        Self { config, client }
    }

    /// 获取底层客户端引用（供需要直接调用 ZLM API 的场景）
    pub fn client(&self) -> &ZlmClient {
        &self.client
    }

    /// 获取媒体服务器 host（不带端口）
    fn media_host(&self) -> String {
        self.config
            .base_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split(':')
            .next()
            .unwrap_or("127.0.0.1")
            .to_string()
    }

    /// 判断 base_url 是否使用 HTTPS
    fn is_https(&self) -> bool {
        self.config.base_url.starts_with("https://")
    }

    /// 获取 HTTP API 端口（用于 HLS/FLV 等 HTTP 协议）
    fn http_port(&self) -> u16 {
        if self.config.http_port != 0 {
            return self.config.http_port;
        }
        self.config
            .base_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split(':')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(if self.is_https() { 443 } else { 8080 })
    }

    /// 判断是否应生成 HTTPS 播放 URL
    fn has_https(&self) -> bool {
        self.config.https_port != 0
    }
}

#[async_trait::async_trait]
impl MediaBackend for ZlmBackend {
    async fn open_rtp_server(&self, req: OpenRtpRequest) -> RIE<RtpServerHandle> {
        let port = self
            .client
            .open_rtp_server(&req.stream_id, req.port, req.tcp_mode.as_zlm_value())
            .await?;

        Ok(RtpServerHandle {
            stream_id: req.stream_id,
            port,
            token: None,
        })
    }

    async fn close_rtp_server(&self, stream_id: &str) -> RIE<()> {
        self.client.close_rtp_server(stream_id).await?;
        Ok(())
    }

    async fn is_stream_online(&self, stream_id: &str) -> bool {
        self.client.is_stream_online("rtp", stream_id).await
    }

    fn get_play_urls(&self, stream_id: &str) -> PlayUrls {
        let host = self.media_host();
        let http_port = self.http_port();
        let rtsp_port = self.config.rtsp_port;
        let rtmp_port = self.config.rtmp_port;

        let mut urls = PlayUrls {
            stream_id: stream_id.to_string(),
            rtsp: format!("rtsp://{}:{}/rtp/{}", host, rtsp_port, stream_id),
            rtsps: String::new(),
            rtmp: format!("rtmp://{}:{}/rtp/{}", host, rtmp_port, stream_id),
            hls: format!("http://{}:{}/rtp/{}/hls.m3u8", host, http_port, stream_id),
            hls_https: String::new(),
            webrtc: format!(
                "webrtc://{}:{}/index/api/webrtc?app=rtp&stream={}",
                host, http_port, stream_id
            ),
            flv: format!(
                "http://{}:{}/rtp/{}.live.flv",
                host, http_port, stream_id
            ),
            flv_https: String::new(),
        };

        // TLS 变体
        if self.config.rtsps_port != 0 {
            urls.rtsps = format!(
                "rtsps://{}:{}/rtp/{}",
                host, self.config.rtsps_port, stream_id
            );
        }
        if self.has_https() {
            let hp = self.config.https_port;
            urls.hls_https = format!("https://{}:{}/rtp/{}/hls.m3u8", host, hp, stream_id);
            urls.flv_https = format!("https://{}:{}/rtp/{}.live.flv", host, hp, stream_id);
        }

        urls
    }

    async fn list_streams(&self) -> RIE<Vec<MediaStreamInfo>> {
        let list = self.client.get_media_list("", "").await?;
        Ok(list
            .into_iter()
            .map(|m| MediaStreamInfo {
                app: m.app,
                stream_id: m.stream,
                schema: m.schema,
                reader_count: m.totalReaderCount.unwrap_or(0),
                alive_secs: m.aliveSecond.unwrap_or(0),
                origin_url: m.originUrl,
            })
            .collect())
    }

    async fn add_stream_proxy(
        &self,
        stream_id: &str,
        source_url: &str,
    ) -> RIE<StreamProxyHandle> {
        let key = self
            .client
            .add_stream_proxy("rtp", stream_id, source_url)
            .await?;

        Ok(StreamProxyHandle {
            key,
            stream_id: stream_id.to_string(),
        })
    }

    async fn remove_stream_proxy(&self, key: &str) -> RIE<bool> {
        Ok(self.client.del_stream_proxy(key).await?)
    }

    fn backend_name(&self) -> &'static str {
        "zlm"
    }

    async fn health_check(&self) -> RIE<()> {
        self.client
            .get_media_list("", "")
            .await
            .map(|_| ())
            .map_err(|e| {
                tx_di_core::IE::WithContext {
                    context: "ZLM 健康检查失败".to_string(),
                    source: e,
                }
            })
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 单元测试
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zlm_backend_config_default() {
        let cfg = ZlmBackendConfig::default();
        assert_eq!(cfg.base_url, "http://127.0.0.1:8080");
        assert_eq!(cfg.rtsp_port, 554);
        assert_eq!(cfg.rtmp_port, 1935);
        assert_eq!(cfg.rtsps_port, 0);
        assert_eq!(cfg.http_port, 0);
        assert_eq!(cfg.https_port, 0);
        assert_eq!(cfg.timeout_secs, 10);
    }

    #[test]
    fn zlm_backend_host_and_ports() {
        let cfg = ZlmBackendConfig {
            base_url: "http://192.168.1.100:8080".to_string(),
            rtsp_port: 554,
            http_port: 0,
            https_port: 8443,
            ..Default::default()
        };
        let backend = ZlmBackend::new(cfg);
        assert_eq!(backend.media_host(), "192.168.1.100");
        assert_eq!(backend.http_port(), 8080);
        assert!(backend.has_https());
        assert!(!backend.is_https());
    }

    #[test]
    fn zlm_backend_explicit_http_port() {
        let cfg = ZlmBackendConfig {
            base_url: "http://127.0.0.1:8080".to_string(),
            http_port: 9090,
            ..Default::default()
        };
        let backend = ZlmBackend::new(cfg);
        assert_eq!(backend.http_port(), 9090); // 显式配置优先
    }

    #[test]
    fn zlm_backend_https_base_url() {
        let cfg = ZlmBackendConfig {
            base_url: "https://127.0.0.1".to_string(),
            http_port: 0,
            https_port: 0,
            ..Default::default()
        };
        let backend = ZlmBackend::new(cfg);
        assert!(backend.is_https());
        assert_eq!(backend.http_port(), 443); // HTTPS 默认端口
    }

    #[test]
    fn zlm_play_urls_basic() {
        let cfg = ZlmBackendConfig {
            base_url: "http://10.0.0.1:8080".to_string(),
            rtsp_port: 554,
            rtmp_port: 1935,
            rtsps_port: 0,
            http_port: 0,
            https_port: 0,
            ..Default::default()
        };
        let backend = ZlmBackend::new(cfg);
        let urls = backend.get_play_urls("ch001");

        assert_eq!(urls.stream_id, "ch001");
        assert_eq!(urls.rtsp, "rtsp://10.0.0.1:554/rtp/ch001");
        assert_eq!(urls.rtmp, "rtmp://10.0.0.1:1935/rtp/ch001");
        assert_eq!(urls.hls, "http://10.0.0.1:8080/rtp/ch001/hls.m3u8");
        assert_eq!(urls.flv, "http://10.0.0.1:8080/rtp/ch001.live.flv");
        assert!(urls.webrtc.contains("ch001"));
        // 未配置 TLS，TLS 字段应为空
        assert!(urls.rtsps.is_empty());
        assert!(urls.hls_https.is_empty());
        assert!(urls.flv_https.is_empty());
    }

    #[test]
    fn zlm_play_urls_with_tls() {
        let cfg = ZlmBackendConfig {
            base_url: "http://10.0.0.1:8080".to_string(),
            rtsp_port: 554,
            rtmp_port: 1935,
            rtsps_port: 322,
            http_port: 8080,
            https_port: 8443,
            ..Default::default()
        };
        let backend = ZlmBackend::new(cfg);
        let urls = backend.get_play_urls("stream1");

        assert_eq!(urls.rtsps, "rtsps://10.0.0.1:322/rtp/stream1");
        assert_eq!(urls.hls_https, "https://10.0.0.1:8443/rtp/stream1/hls.m3u8");
        assert_eq!(urls.flv_https, "https://10.0.0.1:8443/rtp/stream1.live.flv");
    }

    #[test]
    fn zlm_backend_name() {
        let backend = ZlmBackend::new(ZlmBackendConfig::default());
        assert_eq!(backend.backend_name(), "zlm");
    }

    #[test]
    fn zlm_config_serialize_roundtrip() {
        let cfg = ZlmBackendConfig {
            base_url: "http://example.com:9090".to_string(),
            secret: "test-secret".to_string(),
            rtsp_port: 8554,
            rtsps_port: 8322,
            rtmp_port: 1935,
            http_port: 9090,
            https_port: 9443,
            timeout_secs: 30,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: ZlmBackendConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg2.base_url, "http://example.com:9090");
        assert_eq!(cfg2.rtsp_port, 8554);
        assert_eq!(cfg2.https_port, 9443);
    }
}
