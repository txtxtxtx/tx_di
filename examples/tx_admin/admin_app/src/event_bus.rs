//! 进程内领域事件总线
//!
//! 实现 [`DomainEventPublisher`]，应用服务在事务提交后发布领域事件。
//!
//! 设计要点：
//! - 订阅者按事件类型（`TypeId`）路由注册，O(1) 分发，避免线性匹配
//! - 订阅者 panic 被隔离（`catch_unwind`），不影响主流程
//! - 进程内实现，未来可演进为 Outbox / 分布式事件总线（接口不变）

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use admin_domain::shared::event_publisher::DomainEventPublisher;
use admin_domain::shared::model::event::Event;
use tx_di_core::{Component, DepsTuple};

/// 领域事件订阅者回调类型（接收类型擦除后的 `Any` 视图，内部再做 downcast）
type RawSubscriber = Arc<dyn Fn(&dyn Any) + Send + Sync>;

/// 进程内领域事件总线
#[derive(Component)]
#[component(as_trait = dyn DomainEventPublisher)]
pub struct EventBus {
    #[tx_cst(RwLock::new(HashMap::new()))]
    subscribers: RwLock<HashMap<TypeId, Vec<RawSubscriber>>>,
}

impl EventBus {
    /// 创建事件总线
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
        }
    }

    /// 泛型订阅：按事件类型 `E` 路由
    ///
    /// 订阅者在事件发布时被同步调用；请勿在订阅者中执行重操作
    /// （如需异步处理，在订阅者内部 `tokio::spawn`）。
    ///
    /// # 示例
    /// ```ignore
    /// event_bus.on::<UserEvent>(|event| {
    ///     if let UserEvent::UserCreated { user_id, username } = event {
    ///         tracing::info!("用户创建: id={} username={}", user_id, username);
    ///     }
    /// });
    /// ```
    pub fn on<E: Event + Clone>(&self, handler: impl Fn(E) + Send + Sync + 'static) {
        let wrapped: RawSubscriber = Arc::new(move |any| {
            if let Some(e) = any.downcast_ref::<E>() {
                handler(e.clone());
            }
        });
        if let Ok(mut subs) = self.subscribers.write() {
            subs.entry(TypeId::of::<E>()).or_default().push(wrapped);
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainEventPublisher for EventBus {
    fn publish(&self, events: Vec<Arc<dyn Event>>) {
        // 快照订阅者，避免持有读锁期间调用用户回调（防死锁）
        let subs: HashMap<TypeId, Vec<RawSubscriber>> = match self.subscribers.read() {
            Ok(s) => s.clone(),
            Err(_) => return,
        };
        for event in &events {
            let any = event.as_any();
            if let Some(handlers) = subs.get(&any.type_id()) {
                for handler in handlers {
                    // 订阅者异常隔离，不影响发布方主流程
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(any)));
                }
            }
        }
    }
}
