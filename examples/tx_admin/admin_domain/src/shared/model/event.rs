//! 领域事件标记 trait

use std::any::Any;

/// 领域事件标记 trait
///
/// 每个限界上下文（域）定义自己的事件枚举并实现该 trait。
/// 事件总线通过 [`Event::as_any`] 获得的 `TypeId` 做 O(1) 类型路由，
/// 订阅者再 `downcast_ref` 还原为具体事件类型。
pub trait Event: Any + Send + Sync {
    /// 返回事件类型的擦除视图，用于类型分发
    fn as_any(&self) -> &dyn Any;
}
