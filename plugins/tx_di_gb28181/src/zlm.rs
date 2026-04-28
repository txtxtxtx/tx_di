//! ZLMediaServer HTTP API 客户端
//!
//! 封装 ZLM REST API，用于流媒体管理：
//! - `openRtpServer`   — 开启 RTP 接收端口（设备推流到此端口）
//! - `closeRtpServer`  — 关闭 RTP 接收端口
//! - `getMediaList`    — 查询活跃媒体流列表
//! - `addStreamProxy`  — 添加 RTSP/RTMP 拉流代理
//! - `delStreamProxy`  — 删除拉流代理
//! - `getStreamUrl`    — 获取播放 URL（HLS/RTSP/WebRTC等）
//!
//! # ZLM 配置
//! ```toml
//! [gb28181_server_config.zlm]
//! base_url = "http://127.0.0.1:8080"
//! secret   = "035c73f7-bb6b-4889-a715-d9eb2d1925cc"
//! ```

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// ZLM API 响应通用封装
#[derive(Debug, Deserialize)]
struct ZlmResponse<T> {
    code: i32,
    msg: Option<String>,
    #[serde(flatten)]
    data: Option<T>,
}

/// ZLMediaServer 配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZlmConfig {
    /// ZLM HTTP API 地址，例如 "http://127.0.0.1:8080"
    #[serde(default = "default_zlm_url")]
    pub base_url: String,

    /// ZLM API 鉴权 secret
    #[serde(default = "default_zlm_secret")]
    pub secret: String,
}

impl Default for ZlmConfig {
    fn default() -> Self {
        Self {
            base_url: default_zlm_url(),
            secret: default_zlm_secret(),
        }
    }
}

fn default_zlm_url() -> String {
    "http://127.0.0.1:8080".to_string()
}
fn default_zlm_secret() -> String {
    "035c73f7-bb6b-4889-a715-d9eb2d1925cc".to_string()
}

/// 开启 RTP 接收端口的响应
#[derive(Debug, Clone, Deserialize)]
pub struct OpenRtpServerResult {
    /// ZLM 分配的 RTP 端口
    pub port: u16,
}

/// 活跃媒体流信息
#[derive(Debug, Clone, Deserialize)]
pub struct MediaInfo {
    pub app: String,
    pub stream: String,
    pub schema: String,
    pub vhost: String,
    pub originUrl: Option<String>,
    pub aliveSecond: Option<u64>,
    pub totalReaderCount: Option<u32>,
}

/// 流代理响应
#[derive(Debug, Clone, Deserialize)]
pub struct StreamProxyResult {
    pub key: String,
}

/// ZLMediaServer HTTP API 客户端
#[derive(Clone)]
pub struct ZlmClient {
    config: ZlmConfig,
    http: reqwest::Client,
}

impl ZlmClient {
    pub fn new(config: ZlmConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("创建 HTTP 客户端失败");
        Self { config, http }
    }

    // ── 内部工具 ─────────────────────────────────────────────────────────────

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        let url = self.url(path);
        let mut all_params: Vec<(&str, &str)> = vec![("secret", &self.config.secret)];
        all_params.extend_from_slice(params);

        debug!(url = %url, "ZLM GET");

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

        debug!(response = %text, "ZLM 响应");

        let val: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("ZLM JSON 解析失败: {} — 原始: {}", e, text))?;

        let code = val.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = val
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow::anyhow!("ZLM API 错误 (code={}, msg={})", code, msg));
        }

        serde_json::from_value(val)
            .map_err(|e| anyhow::anyhow!("ZLM 响应数据反序列化失败: {}", e))
    }

    // ── 公开 API ─────────────────────────────────────────────────────────────

    /// 开启 RTP 接收端口
    ///
    /// ZLM 会在随机端口（或指定端口）监听 RTP，设备推流到此端口后会自动创建流。
    ///
    /// # 参数
    /// - `stream_id`：流标识，建议用 `channel_id` 或 `call_id`
    /// - `port`：指定端口（0 表示 ZLM 随机分配）
    /// - `tcp_mode`：0=UDP，1=TCP 主动，2=TCP 被动
    ///
    /// # 返回
    /// ZLM 实际监听的端口号
    pub async fn open_rtp_server(
        &self,
        stream_id: &str,
        port: u16,
        tcp_mode: u8,
    ) -> anyhow::Result<u16> {
        #[derive(Deserialize)]
        struct Resp {
            port: u16,
        }

        let port_str = port.to_string();
        let tcp_str = tcp_mode.to_string();

        let val: serde_json::Value = self
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
    ///
    /// 会话结束后必须调用，释放端口资源。
    pub async fn close_rtp_server(&self, stream_id: &str) -> anyhow::Result<bool> {
        let val: serde_json::Value = self
            .get_raw("/index/api/closeRtpServer", &[("stream_id", stream_id)])
            .await?;

        let hit = val
            .get("hit")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(hit > 0)
    }

    /// 查询媒体流列表
    ///
    /// # 参数
    /// - `app`：应用名（为空则查询所有）
    /// - `stream`：流 ID（为空则查询所有）
    pub async fn get_media_list(
        &self,
        app: &str,
        stream: &str,
    ) -> anyhow::Result<Vec<MediaInfo>> {
        #[derive(Deserialize)]
        struct Resp {
            data: Vec<MediaInfo>,
        }

        let val: serde_json::Value = self
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
    ///
    /// 让 ZLM 主动拉取外部流并转发给播放端。
    ///
    /// # 返回
    /// 代理流的 key（删除时使用）
    pub async fn add_stream_proxy(
        &self,
        app: &str,
        stream: &str,
        url: &str,
    ) -> anyhow::Result<String> {
        let val: serde_json::Value = self
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
        let val: serde_json::Value = self
            .get_raw("/index/api/delStreamProxy", &[("key", key)])
            .await?;

        let data = val
            .get("data")
            .and_then(|v| v.get("flag"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(data)
    }

    /// 获取流的各协议播放 URL
    ///
    /// 返回该流在 ZLM 上可用的 RTSP/RTMP/HLS/WebRTC 播放地址。
    pub fn get_play_urls(&self, app: &str, stream_id: &str) -> PlayUrls {
        let host_port = self
            .config
            .base_url
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        // 提取 host（去掉端口）
        let host = host_port
            .split(':')
            .next()
            .unwrap_or("127.0.0.1");

        PlayUrls {
            rtsp: format!("rtsp://{}:554/{}/{}", host, app, stream_id),
            rtmp: format!("rtmp://{}:1935/{}/{}", host, app, stream_id),
            hls: format!("http://{}:8080/{}/{}/hls.m3u8", host, app, stream_id),
            webrtc: format!("webrtc://{}:8080/index/api/webrtc?app={}&stream={}", host, app, stream_id),
            flv: format!("http://{}:8080/{}/{}.live.flv", host, app, stream_id),
        }
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
        let mut all_params: Vec<(String, String)> = vec![("secret".into(), self.config.secret.clone())];
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
            return Err(anyhow::anyhow!("ZLM API 错误 [{}] (code={}, msg={})", path, code, msg));
        }

        Ok(val)
    }
}

/// 各协议播放 URL
#[derive(Debug, Clone)]
pub struct PlayUrls {
    pub rtsp: String,
    pub rtmp: String,
    pub hls: String,
    pub webrtc: String,
    pub flv: String,
}
