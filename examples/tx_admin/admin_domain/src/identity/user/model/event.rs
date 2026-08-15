use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::shared::model::event::Event;
use crate::identity::user::model::value_object::UserStatus;

/// 用户域领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserEvent {
    UserCreated { user_id: u64, username: String },
    UserUpdated { user_id: u64 },
    UserDeleted { user_id: u64 },
    UserStatusChanged { user_id: u64, status: UserStatus },
    UserPasswordChanged { user_id: u64 },
    UserLoggedIn { user_id: u64, ip: String },
}

impl Event for UserEvent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
