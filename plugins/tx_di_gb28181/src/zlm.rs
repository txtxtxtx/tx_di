//! ZLMediaServer 客户端类型入口（兼容旧版引用）
//!
//! 所有 ZLM 相关类型的实际实现已统一到 [`crate::media::zlm`]。
//! 本模块仅做 re-export，保持 `crate::zlm::ZlmClient` 等旧路径可用。
//!
//! > **新代码应直接使用 `crate::media::zlm::*`。**

// re-export 所有公开类型
pub use crate::media::zlm::{MediaInfo, ZlmBackend, ZlmBackendConfig, ZlmClient};

// ── 兼容旧版配置类型 ──────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// 旧版简化 ZLM 配置（只有 base_url + secret）
///
/// 新代码请使用 [`ZlmBackendConfig`]，它包含完整的端口配置。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZlmConfig {
    pub base_url: String,
    pub secret: String,
}

impl Default for ZlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080".to_string(),
            secret: "035c73f7-bb6b-4889-a715-d9eb2d1925cc".to_string(),
        }
    }
}

impl From<ZlmConfig> for ZlmBackendConfig {
    fn from(cfg: ZlmConfig) -> Self {
        Self {
            base_url: cfg.base_url,
            secret: cfg.secret,
            ..Default::default()
        }
    }
}

impl From<ZlmBackendConfig> for ZlmConfig {
    fn from(cfg: ZlmBackendConfig) -> Self {
        Self {
            base_url: cfg.base_url,
            secret: cfg.secret,
        }
    }
}
