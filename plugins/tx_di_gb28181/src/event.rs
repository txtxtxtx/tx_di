//! GB28181 事件总线（服务端插件）
//!
//! 事件类型（`Gb28181Event`）和广播基础设施已迁移到 `tx_gb28181::event` 公共模块，
//! 此处通过 re-export 保持内部路径兼容。

// ── 从公共模块 re-export（向后兼容）─────────────────────────────────────────
pub use tx_gb28181::event::{Gb28181Event, add_event_listener, emit, subscribe};
