//! 进程内领域事件总线
//!
//! 实现 [`DomainEventPublisher`]，应用服务在事务提交后发布领域事件。
//!
//! 设计要点：
//! - 订阅者以回调注册（同步投递，fire-and-forget）
//! - 订阅者 panic 被隔离（`catch_unwind`），不影响主流程
//! - 进程内实现，未来可演进为 Outbox / 分布式事件总线（接口不变）

use std::sync::{Arc, RwLock};

use admin_domain::shared::event_publisher::DomainEventPublisher;
use admin_domain::shared::model::DomainEvent;
use tx_di_core::{Component, DepsTuple};

/// 领域事件订阅者回调类型
type Subscriber = Arc<dyn Fn(DomainEvent) + Send + Sync>;

/// 进程内领域事件总线
#[derive(Component)]
#[component(as_trait = dyn DomainEventPublisher)]
pub struct EventBus {
    #[tx_cst(RwLock::new(Vec::new()))]
    subscribers: RwLock<Vec<Subscriber>>,
}

impl EventBus {
    /// 创建事件总线
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(Vec::new()),
        }
    }

    /// 注册事件订阅者
    ///
    /// 订阅者在事件发布时被同步调用；请勿在订阅者中执行重操作
    /// （如需异步处理，在订阅者内部 `tokio::spawn`）。
    pub fn subscribe(&self, handler: impl Fn(DomainEvent) + Send + Sync + 'static) {
        if let Ok(mut subs) = self.subscribers.write() {
            subs.push(Arc::new(handler));
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainEventPublisher for EventBus {
    fn publish(&self, events: Vec<DomainEvent>) {
        // 快照订阅者，避免持有读锁期间调用用户回调（防死锁）
        let subs: Vec<Arc<dyn Fn(DomainEvent) + Send + Sync>> = match self.subscribers.read() {
            Ok(s) => s.iter().cloned().collect(),
            Err(_) => return,
        };
        for event in events {
            for sub in &subs {
                // 订阅者异常隔离，不影响发布方主流程
                let _ =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sub(event.clone())));
            }
        }
    }
}
