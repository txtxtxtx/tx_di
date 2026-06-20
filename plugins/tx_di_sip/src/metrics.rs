//! SIP 运行时指标
//!
//! 提供 `SipPlugin` 的运行状态快照，可用于健康检查或监控接入。

use serde::Serialize;

/// SIP 插件运行时指标
#[derive(Clone, Debug, Serialize)]
pub struct SipMetrics {
    /// 插件是否处于运行状态
    pub running: bool,

    /// 已注册的 SIP 方法处理器数量
    pub handler_count: usize,

    /// 已注册的 SIP 方法名列表
    pub registered_methods: Vec<String>,

    /// 服务连续运行时长（秒），未启动时为 0
    pub uptime_secs: u64,
}
