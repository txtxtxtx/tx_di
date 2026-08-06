//! 领域事件发布器接口
//!
//! 接口定义在 domain 层（依赖倒置），由应用层 / 基础设施层提供实现。
//! 聚合根通过 [`crate::shared::model::AggregateRoot::add_event`] 收集事件，
//! 应用服务在**持久化成功（事务提交）后**调用发布器投递。

use crate::shared::model::DomainEvent;

/// 领域事件发布器
pub trait DomainEventPublisher: Send + Sync {
    /// 发布一批领域事件
    ///
    /// 约定：投递失败不应影响主流程（fire-and-forget），
    /// 实现方需保证订阅者异常被隔离。
    fn publish(&self, events: Vec<DomainEvent>);
}
