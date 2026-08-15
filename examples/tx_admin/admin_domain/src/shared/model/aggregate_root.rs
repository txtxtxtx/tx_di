//! 聚合根基类 trait（泛型化事件类型）

use crate::shared::model::entity::Entity;
use crate::shared::model::event::Event;

/// 聚合根基类
///
/// 聚合根在其方法中通过 [`AggregateRoot::add_event`] 收集领域事件，
/// 应用服务在事务提交后调用
/// [`crate::shared::event_publisher::DomainEventPublisher`] 投递。
pub trait AggregateRoot<E: Event>: Entity {
    /// 获取待发布的领域事件
    fn events(&self) -> &[E];
    /// 清空待发布的领域事件
    fn clear_events(&mut self);
    /// 追加一条领域事件
    fn add_event(&mut self, event: E);
}
