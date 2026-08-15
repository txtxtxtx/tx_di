//! 领域事件发布器接口
//!
//! 接口定义在 domain 层（依赖倒置），由应用层 / 基础设施层提供实现。
//! 聚合根通过 [`crate::shared::model::AggregateRoot::add_event`] 收集事件，
//! 应用服务在**持久化成功（事务提交）后**调用发布器投递。

use std::sync::Arc;

use crate::shared::model::event::Event;

/// 领域事件发布器（dyn 兼容）
///
/// 事件以 `Arc<dyn Event>` 类型擦除发布；订阅方按 `TypeId` 路由后再
/// `downcast_ref` 还原具体事件类型。
pub trait DomainEventPublisher: Send + Sync {
    /// 发布一批领域事件（类型擦除为 `Arc<dyn Event>`）
    ///
    /// 约定：投递失败不应影响主流程（fire-and-forget），
    /// 实现方需保证订阅者异常被隔离。
    fn publish(&self, events: Vec<Arc<dyn Event>>);
}

/// 便捷函数：将具体类型事件擦除为 `Arc<dyn Event>` 后发布
pub fn publish_typed<E: Event>(publisher: &dyn DomainEventPublisher, events: Vec<E>) {
    publisher.publish(events.into_iter().map(|e| Arc::new(e) as Arc<dyn Event>).collect());
}
